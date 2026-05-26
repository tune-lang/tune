use tune_runtime::value::Value;

pub struct Frame {
    pub registers: Vec<Value>,
    pub ip: usize,
    pub function: usize,
}
