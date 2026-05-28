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
    Range(RangeValue),
    Struct {
        owner: u32,
        state: StateHandle,
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
            (Self::Range(left), Self::Range(right)) => left == right,
            (
                Self::Struct {
                    owner: left_owner,
                    state: left_state,
                    fields: left_fields,
                },
                Self::Struct {
                    owner: right_owner,
                    state: right_state,
                    fields: right_fields,
                },
            ) => {
                left_owner == right_owner
                    && left_state == right_state
                    && left_fields == right_fields
            }
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
pub struct StructFields(Rc<RefCell<Vec<Value>>>);

impl StructFields {
    #[must_use]
    pub fn new(fields: Vec<Value>) -> Self {
        Self(Rc::new(RefCell::new(fields)))
    }

    #[must_use]
    pub fn get(&self, index: usize) -> Option<Value> {
        self.0.borrow().get(index).cloned()
    }

    pub fn set(&self, index: usize, value: Value) -> Option<()> {
        let mut fields = self.0.borrow_mut();
        let field = fields.get_mut(index)?;
        *field = value;
        Some(())
    }
}

impl PartialEq for StructFields {
    fn eq(&self, other: &Self) -> bool {
        *self.0.borrow() == *other.0.borrow()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CallableValue(pub u64);

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
