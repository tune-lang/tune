use std::sync::{Arc, Mutex};

use crate::resource::ResourceHandle;
use crate::sequence::SequenceHandle;
use crate::state::StateHandle;
use crate::task::TaskId;
use tune_diagnostics::Span;

#[derive(Debug, Clone)]
pub enum Value {
    /// Boxed execution/debug value used by the current VM and host boundary.
    ///
    /// Tune's performance contract is the typed plan/IR/bytecode pipeline, not
    /// dynamic dispatch over this enum. Future optimized VM/native backends
    /// should use typed register storage or specialized lanes for hot ops.
    Unit,
    None,
    Int(i64),
    Float(f64),
    Size(u64),
    Byte(u8),
    Bool(bool),
    String(String),
    /// Host/debug sequence value. VM-built sequences use `SequenceHandle`.
    Sequence(Vec<Value>),
    /// Current VM sequence storage with explicit exclusive/shared COW mutation.
    SequenceHandle(SequenceHandle),
    Tuple(Vec<Value>),
    Range(RangeValue),
    Struct {
        owner: u32,
        fields: StructFields,
    },
    HostStruct {
        type_name: String,
        fields: Vec<(String, Value)>,
    },
    Variant {
        variant: RuntimeVariant,
        fields: Vec<Value>,
        propagation_frames: Vec<PropagationFrame>,
    },
    StructState(StateHandle),
    Resource(ResourceHandle),
    Callable(CallableValue),
    Task(TaskHandle),
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Unit, Self::Unit) => true,
            (Self::None, Self::None) => true,
            (Self::Int(left), Self::Int(right)) => left == right,
            (Self::Float(left), Self::Float(right)) => left == right,
            (Self::Size(left), Self::Size(right)) => left == right,
            (Self::Byte(left), Self::Byte(right)) => left == right,
            (Self::Bool(left), Self::Bool(right)) => left == right,
            (Self::String(left), Self::String(right)) => left == right,
            (Self::Sequence(left), Self::Sequence(right)) => left == right,
            (Self::SequenceHandle(left), Self::SequenceHandle(right)) => left == right,
            (Self::Sequence(left), Self::SequenceHandle(right)) => {
                left.len() == right.len()
                    && left
                        .iter()
                        .enumerate()
                        .all(|(index, value)| right.get(index).is_some_and(|right| right == *value))
            }
            (Self::SequenceHandle(left), Self::Sequence(right)) => {
                right.len() == left.len()
                    && right
                        .iter()
                        .enumerate()
                        .all(|(index, value)| left.get(index).is_some_and(|left| left == *value))
            }
            (Self::Tuple(left), Self::Tuple(right)) => left == right,
            (Self::Range(left), Self::Range(right)) => left == right,
            (
                Self::Struct {
                    owner: left_owner,
                    fields: left_fields,
                },
                Self::Struct {
                    owner: right_owner,
                    fields: right_fields,
                },
            ) => left_owner == right_owner && left_fields == right_fields,
            (
                Self::HostStruct {
                    type_name: left_type,
                    fields: left_fields,
                },
                Self::HostStruct {
                    type_name: right_type,
                    fields: right_fields,
                },
            ) => left_type == right_type && left_fields == right_fields,
            (
                Self::Variant {
                    variant: left_variant,
                    fields: left_fields,
                    propagation_frames: left_frames,
                },
                Self::Variant {
                    variant: right_variant,
                    fields: right_fields,
                    propagation_frames: right_frames,
                },
            ) => {
                left_variant == right_variant
                    && left_fields == right_fields
                    && left_frames == right_frames
            }
            (Self::StructState(left), Self::StructState(right)) => left == right,
            (Self::Resource(left), Self::Resource(right)) => left == right,
            (Self::Callable(left), Self::Callable(right)) => left == right,
            (Self::Task(left), Self::Task(right)) => left == right,
            _ => false,
        }
    }
}

impl Value {
    #[must_use]
    pub fn capture_snapshot(&self) -> Self {
        match self {
            // A struct snapshot needs a fresh state identity. The VM owns state
            // allocation, so use `Vm::capture_snapshot` for executable closure
            // and task captures.
            Self::Struct { .. } => self.clone(),
            Self::HostStruct { type_name, fields } => Self::HostStruct {
                type_name: type_name.clone(),
                fields: fields
                    .iter()
                    .map(|(name, value)| (name.clone(), value.capture_snapshot()))
                    .collect(),
            },
            Self::Sequence(values) => {
                Self::Sequence(values.iter().map(Self::capture_snapshot).collect())
            }
            Self::SequenceHandle(values) => Self::Sequence(values.snapshot_values()),
            Self::Tuple(values) => Self::Tuple(values.iter().map(Self::capture_snapshot).collect()),
            Self::Variant {
                variant,
                fields,
                propagation_frames,
            } => Self::Variant {
                variant: *variant,
                fields: fields.iter().map(Self::capture_snapshot).collect(),
                propagation_frames: propagation_frames.clone(),
            },
            value => value.clone(),
        }
    }

    #[must_use]
    pub fn task_safety_error(&self) -> Option<TaskSafetyError> {
        match self {
            Self::Resource(resource) if !resource.task_safe => Some(TaskSafetyError {
                resource_type: resource.type_name.clone(),
            }),
            Self::Sequence(values) | Self::Tuple(values) => {
                values.iter().find_map(Self::task_safety_error)
            }
            Self::SequenceHandle(values) => values.task_safety_error(),
            Self::Struct { fields, .. } => fields.task_safety_error(),
            Self::HostStruct { fields, .. } => fields
                .iter()
                .find_map(|(_, value)| value.task_safety_error()),
            Self::Variant { fields, .. } => fields.iter().find_map(Self::task_safety_error),
            Self::Callable(callable) => callable.task_safety_error(),
            _ => None,
        }
    }

    #[must_use]
    pub fn is_task_safe(&self) -> bool {
        self.task_safety_error().is_none()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RangeValue {
    pub start: i128,
    pub end: i128,
    pub inclusive: bool,
    pub item: RangeItemKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RangeItemKind {
    Int,
    Size,
}

#[derive(Debug, Clone)]
pub struct StructFields {
    state: StateHandle,
    // Current debug/shared-handle storage. This is not the final optimized
    // state model: ownership plans decide stack/direct-drop/RC/COW/shared
    // placement, and only genuinely shared state should pay synchronization.
    fields: Arc<Mutex<Vec<Value>>>,
}

impl StructFields {
    #[must_use]
    pub fn new(state: StateHandle, fields: Vec<Value>) -> Self {
        Self {
            state,
            fields: Arc::new(Mutex::new(fields)),
        }
    }

    #[must_use]
    pub const fn state(&self) -> StateHandle {
        self.state
    }

    #[must_use]
    pub fn get(&self, index: usize) -> Option<Value> {
        self.fields
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(index)
            .cloned()
    }

    pub fn set(&self, index: usize, value: Value) -> Option<()> {
        let mut fields = self
            .fields
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let field = fields.get_mut(index)?;
        *field = value;
        Some(())
    }

    #[must_use]
    pub fn snapshot_with_state(&self, state: StateHandle) -> Self {
        let fields = self
            .fields
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();
        Self::new(state, fields.iter().map(Value::capture_snapshot).collect())
    }

    #[must_use]
    pub fn task_safety_error(&self) -> Option<TaskSafetyError> {
        self.fields
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .iter()
            .find_map(Value::task_safety_error)
    }
}

impl PartialEq for StructFields {
    fn eq(&self, other: &Self) -> bool {
        let left = self
            .fields
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let right = other
            .fields
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        self.state == other.state && *left == *right
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CallableValue {
    pub function: u32,
    pub captures: Vec<CapturedValue>,
}

impl CallableValue {
    #[must_use]
    pub fn task_safety_error(&self) -> Option<TaskSafetyError> {
        self.captures
            .iter()
            .map(CapturedValue::get)
            .find_map(|value| value.task_safety_error())
    }
}

#[derive(Debug, Clone)]
pub struct CapturedValue {
    mode: CaptureStorageMode,
    value: Arc<Mutex<Value>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureStorageMode {
    Reference,
    PrivateSnapshot,
}

impl CapturedValue {
    #[must_use]
    pub fn new(value: Value, mode: CaptureStorageMode) -> Self {
        Self {
            mode,
            value: Arc::new(Mutex::new(value)),
        }
    }

    #[must_use]
    pub const fn mode(&self) -> CaptureStorageMode {
        self.mode
    }

    #[must_use]
    pub fn get(&self) -> Value {
        self.value
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    pub fn set(&self, value: Value) {
        let mut slot = self
            .value
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *slot = value;
    }
}

impl PartialEq for CapturedValue {
    fn eq(&self, other: &Self) -> bool {
        let left = self
            .value
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let right = other
            .value
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        self.mode == other.mode && *left == *right
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TaskHandle(pub TaskId);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RuntimeVariant {
    ResultOk,
    ResultError,
    Other { owner: u32, index: u32 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropagationFrame {
    pub function: u32,
    pub instruction: u32,
    pub function_name: String,
    pub span: Option<Span>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskSafetyError {
    pub resource_type: String,
}
