use crate::value::Value;

pub struct Task {
    pub result: Option<Value>,
}

impl Task {
    pub fn join(self) -> Value {
        self.result.unwrap_or(Value::Unit)
    }
}
