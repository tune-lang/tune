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
    StructState(StateHandle),
    Callable(CallableValue),
    Task(TaskHandle),
}

#[derive(Debug, Clone)]
pub struct StateHandle(pub u64);

#[derive(Debug, Clone)]
pub struct CallableValue(pub u64);

#[derive(Debug, Clone)]
pub struct TaskHandle(pub u64);
