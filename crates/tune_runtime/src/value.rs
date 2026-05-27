use crate::state::StateHandle;
use crate::task::TaskId;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Unit,
    Int(i64),
    Float(f64),
    Size(u64),
    Byte(u8),
    Bool(bool),
    String(String),
    Sequence(Vec<Value>),
    StructState(StateHandle),
    Callable(CallableValue),
    Task(TaskHandle),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CallableValue(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TaskHandle(pub TaskId);
