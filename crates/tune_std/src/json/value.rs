use serde_json::{Map, Number};
use tune_host::HostCallError;
use tune_runtime::Value;
use tune_shape::Shape;

pub(super) const VALUE_TYPE: &str = "json.Value";
const FIELD_TYPE: &str = "json.Field";

pub(super) fn transform_json_text(
    text: &str,
    render: fn(&serde_json::Value) -> serde_json::Result<String>,
) -> Result<Value, HostCallError> {
    match serde_json::from_str::<serde_json::Value>(text) {
        Ok(value) => match render(&value) {
            Ok(rendered) => Ok(crate::result_ok(Value::String(rendered))),
            Err(error) => Ok(crate::result_error(error.to_string())),
        },
        Err(error) => Ok(crate::result_error(error.to_string())),
    }
}

pub(super) fn json_to_tune(value: &serde_json::Value) -> Result<Value, HostCallError> {
    match value {
        serde_json::Value::Null => Ok(json_null()),
        serde_json::Value::Bool(value) => Ok(json_bool(*value)),
        serde_json::Value::Number(value) => {
            let Some(value) = value.as_f64() else {
                return Err(HostCallError::new(
                    "JSON number cannot be represented as Float",
                ));
            };
            Ok(json_number(value))
        }
        serde_json::Value::String(value) => Ok(json_string(value)),
        serde_json::Value::Array(items) => Ok(json_array(
            items
                .iter()
                .map(json_to_tune)
                .collect::<Result<Vec<_>, _>>()?,
        )),
        serde_json::Value::Object(fields) => Ok(json_object(
            fields
                .iter()
                .map(|(name, value)| json_to_tune(value).map(|value| json_field(name, value)))
                .collect::<Result<Vec<_>, _>>()?,
        )),
    }
}

pub(super) fn tune_to_json(value: &Value) -> Result<serde_json::Value, HostCallError> {
    let fields = host_struct_fields(value, VALUE_TYPE, "value")?;
    match required_string_field(fields, "kind")?.as_str() {
        "null" => Ok(serde_json::Value::Null),
        "bool" => Ok(serde_json::Value::Bool(required_bool_field(
            fields, "bool",
        )?)),
        "number" => {
            let number = required_float_field(fields, "number")?;
            let Some(number) = Number::from_f64(number) else {
                return Err(HostCallError::new("json number must be finite"));
            };
            Ok(serde_json::Value::Number(number))
        }
        "string" => Ok(serde_json::Value::String(required_string_field(
            fields, "string",
        )?)),
        "array" => {
            let items = required_sequence_field(fields, "items")?;
            items
                .iter()
                .map(tune_to_json)
                .collect::<Result<Vec<_>, _>>()
                .map(serde_json::Value::Array)
        }
        "object" => {
            let mut object = Map::new();
            for field in required_sequence_field(fields, "fields")? {
                let field_fields = host_struct_fields(field, FIELD_TYPE, "field")?;
                object.insert(
                    required_string_field(field_fields, "name")?,
                    tune_to_json(required_value_field(field_fields, "value")?)?,
                );
            }
            Ok(serde_json::Value::Object(object))
        }
        kind => Err(HostCallError::new(format!(
            "unknown json.Value kind `{kind}`"
        ))),
    }
}

pub(super) fn json_null() -> Value {
    json_value(
        "null",
        Value::None,
        Value::None,
        Value::None,
        Value::None,
        Value::None,
    )
}

pub(super) fn json_bool(value: bool) -> Value {
    json_value(
        "bool",
        Value::Bool(value),
        Value::None,
        Value::None,
        Value::None,
        Value::None,
    )
}

pub(super) fn json_number(value: f64) -> Value {
    json_value(
        "number",
        Value::None,
        Value::Float(value),
        Value::None,
        Value::None,
        Value::None,
    )
}

pub(super) fn json_string(value: &str) -> Value {
    json_value(
        "string",
        Value::None,
        Value::None,
        Value::String(value.to_owned()),
        Value::None,
        Value::None,
    )
}

pub(super) fn json_array(items: Vec<Value>) -> Value {
    json_value(
        "array",
        Value::None,
        Value::None,
        Value::None,
        Value::Sequence(items),
        Value::None,
    )
}

pub(super) fn json_object(fields: Vec<Value>) -> Value {
    json_value(
        "object",
        Value::None,
        Value::None,
        Value::None,
        Value::None,
        Value::Sequence(fields),
    )
}

fn json_value(
    kind: &str,
    bool_value: Value,
    number: Value,
    string: Value,
    items: Value,
    fields: Value,
) -> Value {
    Value::HostStruct {
        type_name: VALUE_TYPE.into(),
        fields: vec![
            ("kind".into(), Value::String(kind.into())),
            ("bool".into(), bool_value),
            ("number".into(), number),
            ("string".into(), string),
            ("items".into(), items),
            ("fields".into(), fields),
        ],
    }
}

pub(super) fn json_field(name: &str, value: Value) -> Value {
    Value::HostStruct {
        type_name: FIELD_TYPE.into(),
        fields: vec![
            ("name".into(), Value::String(name.into())),
            ("value".into(), value),
        ],
    }
}

pub(super) fn value_shape() -> Shape {
    Shape::Struct(VALUE_TYPE.into())
}

pub(super) fn field_shape() -> Shape {
    Shape::Struct(FIELD_TYPE.into())
}

pub(super) fn result_shape(ok: Shape) -> Shape {
    Shape::Result {
        ok: Box::new(ok),
        err: Box::new(Shape::String),
    }
}

pub(super) fn string_result_shape() -> Shape {
    result_shape(Shape::String)
}

pub(super) fn value_arg<'a>(
    args: &'a [Value],
    index: usize,
    name: &str,
) -> Result<&'a Value, HostCallError> {
    let Some(value) = args.get(index) else {
        return Err(HostCallError::new(format!(
            "missing argument `{name}` at index {index}"
        )));
    };
    host_struct_fields(value, VALUE_TYPE, name)?;
    Ok(value)
}

pub(super) fn value_sequence_arg<'a>(
    args: &'a [Value],
    index: usize,
    name: &str,
) -> Result<&'a [Value], HostCallError> {
    match args.get(index) {
        Some(Value::Sequence(values)) => {
            for value in values {
                host_struct_fields(value, VALUE_TYPE, name)?;
            }
            Ok(values)
        }
        None => Err(HostCallError::new(format!(
            "missing argument `{name}` at index {index}"
        ))),
        _ => Err(HostCallError::new(format!(
            "expected [json.Value] for `{name}`"
        ))),
    }
}

pub(super) fn field_sequence_arg<'a>(
    args: &'a [Value],
    index: usize,
    name: &str,
) -> Result<&'a [Value], HostCallError> {
    match args.get(index) {
        Some(Value::Sequence(values)) => {
            for value in values {
                host_struct_fields(value, FIELD_TYPE, name)?;
            }
            Ok(values)
        }
        None => Err(HostCallError::new(format!(
            "missing argument `{name}` at index {index}"
        ))),
        _ => Err(HostCallError::new(format!(
            "expected [json.Field] for `{name}`"
        ))),
    }
}

pub(super) fn field_arg<'a>(
    args: &'a [Value],
    index: usize,
    name: &str,
) -> Result<&'a [(String, Value)], HostCallError> {
    let Some(value) = args.get(index) else {
        return Err(HostCallError::new(format!(
            "missing argument `{name}` at index {index}"
        )));
    };
    host_struct_fields(value, FIELD_TYPE, name)
}

pub(super) fn optional_json_field(
    fields: &[(String, Value)],
    name: &str,
) -> Result<Value, HostCallError> {
    Ok(required_value_field(fields, name)?.clone())
}

pub(super) fn bool_arg(args: &[Value], index: usize, name: &str) -> Result<bool, HostCallError> {
    match args.get(index) {
        Some(Value::Bool(value)) => Ok(*value),
        None => Err(HostCallError::new(format!(
            "missing argument `{name}` at index {index}"
        ))),
        _ => Err(HostCallError::new(format!("expected Bool for `{name}`"))),
    }
}

pub(super) fn float_arg(args: &[Value], index: usize, name: &str) -> Result<f64, HostCallError> {
    match args.get(index) {
        Some(Value::Float(value)) => Ok(*value),
        None => Err(HostCallError::new(format!(
            "missing argument `{name}` at index {index}"
        ))),
        _ => Err(HostCallError::new(format!("expected Float for `{name}`"))),
    }
}

pub(super) fn host_struct_fields<'a>(
    value: &'a Value,
    type_name: &str,
    name: &str,
) -> Result<&'a [(String, Value)], HostCallError> {
    match value {
        Value::HostStruct {
            type_name: actual,
            fields,
        } if actual == type_name => Ok(fields),
        _ => Err(HostCallError::new(format!(
            "expected {type_name} for `{name}`"
        ))),
    }
}

pub(super) fn required_value_field<'a>(
    fields: &'a [(String, Value)],
    name: &str,
) -> Result<&'a Value, HostCallError> {
    fields
        .iter()
        .find(|(field_name, _)| field_name == name)
        .map(|(_, value)| value)
        .ok_or_else(|| HostCallError::new(format!("missing json field `{name}`")))
}

pub(super) fn required_string_field(
    fields: &[(String, Value)],
    name: &str,
) -> Result<String, HostCallError> {
    match required_value_field(fields, name)? {
        Value::String(value) => Ok(value.clone()),
        Value::None => Err(HostCallError::new(format!("missing json string `{name}`"))),
        _ => Err(HostCallError::new(format!(
            "expected String field `{name}`"
        ))),
    }
}

fn required_bool_field(fields: &[(String, Value)], name: &str) -> Result<bool, HostCallError> {
    match required_value_field(fields, name)? {
        Value::Bool(value) => Ok(*value),
        Value::None => Err(HostCallError::new(format!("missing json bool `{name}`"))),
        _ => Err(HostCallError::new(format!("expected Bool field `{name}`"))),
    }
}

fn required_float_field(fields: &[(String, Value)], name: &str) -> Result<f64, HostCallError> {
    match required_value_field(fields, name)? {
        Value::Float(value) => Ok(*value),
        Value::None => Err(HostCallError::new(format!("missing json number `{name}`"))),
        _ => Err(HostCallError::new(format!("expected Float field `{name}`"))),
    }
}

fn required_sequence_field<'a>(
    fields: &'a [(String, Value)],
    name: &str,
) -> Result<&'a [Value], HostCallError> {
    match required_value_field(fields, name)? {
        Value::Sequence(values) => Ok(values),
        Value::None => Err(HostCallError::new(format!(
            "missing json sequence `{name}`"
        ))),
        _ => Err(HostCallError::new(format!(
            "expected sequence field `{name}`"
        ))),
    }
}
