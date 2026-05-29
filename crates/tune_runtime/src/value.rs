use std::cell::RefCell;
use std::rc::Rc;

use crate::state::StateHandle;
use crate::task::TaskId;
use tune_diagnostics::Span;

#[derive(Debug, Clone)]
pub enum Value {
    Unit,
    Int(i64),
    Float(f64),
    Size(u64),
    Byte(u8),
    Bool(bool),
    String(String),
    Sequence(Vec<Value>),
    Tuple(Vec<Value>),
    Range(RangeValue),
    Struct {
        owner: u32,
        fields: StructFields,
    },
    Variant {
        variant: RuntimeVariant,
        fields: Vec<Value>,
        propagation_frames: Vec<PropagationFrame>,
    },
    StructState(StateHandle),
    Callable(CallableValue),
    Task(TaskHandle),
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Unit, Self::Unit) => true,
            (Self::Int(left), Self::Int(right)) => left == right,
            (Self::Float(left), Self::Float(right)) => left == right,
            (Self::Size(left), Self::Size(right)) => left == right,
            (Self::Byte(left), Self::Byte(right)) => left == right,
            (Self::Bool(left), Self::Bool(right)) => left == right,
            (Self::String(left), Self::String(right)) => left == right,
            (Self::Sequence(left), Self::Sequence(right)) => left == right,
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
            Self::Struct { owner, fields } => Self::Struct {
                owner: *owner,
                fields: fields.snapshot(),
            },
            Self::Sequence(values) => {
                Self::Sequence(values.iter().map(Self::capture_snapshot).collect())
            }
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
    fields: Rc<RefCell<Vec<Value>>>,
}

impl StructFields {
    #[must_use]
    pub fn new(state: StateHandle, fields: Vec<Value>) -> Self {
        Self {
            state,
            fields: Rc::new(RefCell::new(fields)),
        }
    }

    #[must_use]
    pub const fn state(&self) -> StateHandle {
        self.state
    }

    #[must_use]
    pub fn get(&self, index: usize) -> Option<Value> {
        self.fields.borrow().get(index).cloned()
    }

    pub fn set(&self, index: usize, value: Value) -> Option<()> {
        let mut fields = self.fields.borrow_mut();
        let field = fields.get_mut(index)?;
        *field = value;
        Some(())
    }

    #[must_use]
    pub fn snapshot(&self) -> Self {
        Self::new(
            self.state,
            self.fields
                .borrow()
                .iter()
                .map(Value::capture_snapshot)
                .collect(),
        )
    }

    #[must_use]
    pub fn snapshot_with_state(&self, state: StateHandle) -> Self {
        Self::new(
            state,
            self.fields
                .borrow()
                .iter()
                .map(Value::capture_snapshot)
                .collect(),
        )
    }
}

impl PartialEq for StructFields {
    fn eq(&self, other: &Self) -> bool {
        self.state == other.state && *self.fields.borrow() == *other.fields.borrow()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CallableValue {
    pub function: u32,
    pub captures: Vec<CapturedValue>,
}

#[derive(Debug, Clone)]
pub struct CapturedValue {
    mode: CaptureStorageMode,
    value: Rc<RefCell<Value>>,
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
            value: Rc::new(RefCell::new(value)),
        }
    }

    #[must_use]
    pub const fn mode(&self) -> CaptureStorageMode {
        self.mode
    }

    #[must_use]
    pub fn get(&self) -> Value {
        self.value.borrow().clone()
    }

    pub fn set(&self, value: Value) {
        *self.value.borrow_mut() = value;
    }
}

impl PartialEq for CapturedValue {
    fn eq(&self, other: &Self) -> bool {
        self.mode == other.mode && *self.value.borrow() == *other.value.borrow()
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
