use tune_bytecode::Opcode;
use tune_runtime::value::Value;

pub fn execute_binary(op: Opcode, a: &Value, b: &Value) -> Option<Value> {
    match (op, a, b) {
        (Opcode::AddInt, Value::Int(x), Value::Int(y)) => Some(Value::Int(x + y)),
        (Opcode::AddFloat, Value::Float(x), Value::Float(y)) => Some(Value::Float(x + y)),
        (Opcode::AddByteWrap, Value::Byte(x), Value::Byte(y)) => {
            Some(Value::Byte(x.wrapping_add(*y)))
        }
        (Opcode::GreaterInt, Value::Int(x), Value::Int(y)) => Some(Value::Bool(x > y)),
        (Opcode::EqualInt, Value::Int(x), Value::Int(y)) => Some(Value::Bool(x == y)),
        (Opcode::NotEqualInt, Value::Int(x), Value::Int(y)) => Some(Value::Bool(x != y)),
        (Opcode::LessInt, Value::Int(x), Value::Int(y)) => Some(Value::Bool(x < y)),
        (Opcode::LessEqualInt, Value::Int(x), Value::Int(y)) => Some(Value::Bool(x <= y)),
        (Opcode::GreaterEqualInt, Value::Int(x), Value::Int(y)) => Some(Value::Bool(x >= y)),
        _ => None,
    }
}

pub fn execute_unary(op: Opcode, value: &Value) -> Option<Value> {
    match (op, value) {
        (Opcode::NegInt, Value::Int(value)) => value.checked_neg().map(Value::Int),
        (Opcode::NotBool, Value::Bool(value)) => Some(Value::Bool(!value)),
        _ => None,
    }
}
