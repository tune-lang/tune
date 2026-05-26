use tune_bytecode::Opcode;
use tune_runtime::value::Value;

pub fn execute_binary(op: Opcode, a: &Value, b: &Value) -> Option<Value> {
    match (op, a, b) {
        (Opcode::AddInt, Value::Int(x), Value::Int(y)) => Some(Value::Int(x + y)),
        (Opcode::AddFloat, Value::Float(x), Value::Float(y)) => Some(Value::Float(x + y)),
        (Opcode::AddByteWrap, Value::Byte(x), Value::Byte(y)) => {
            Some(Value::Byte(x.wrapping_add(*y)))
        }
        _ => None,
    }
}
