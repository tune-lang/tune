use serde_json::{Map, Number};
use tune_host::{
    HostCallError, HostFunction, HostModule, HostParam, HostValueField, HostValueType,
};
use tune_runtime::Value;
use tune_shape::Shape;

const VALUE_TYPE: &str = "json.Value";
const FIELD_TYPE: &str = "json.Field";

#[must_use]
pub fn install() -> HostModule {
    HostModule::new(
        "json",
        vec![
            HostFunction::new(
                "valid",
                vec![HostParam::new("text", Shape::String)],
                Shape::Bool,
            )
            .task_safe(true)
            .with_executor(|args: &[Value]| {
                let text = crate::string_arg(args, 0, "text")?;
                Ok(Value::Bool(
                    serde_json::from_str::<serde_json::Value>(text).is_ok(),
                ))
            }),
            HostFunction::new(
                "decode",
                vec![HostParam::new("text", Shape::String)],
                result_shape(value_shape()),
            )
            .task_safe(true)
            .with_executor(|args: &[Value]| {
                let text = crate::string_arg(args, 0, "text")?;
                match serde_json::from_str::<serde_json::Value>(text) {
                    Ok(value) => json_to_tune(&value)
                        .map(crate::result_ok)
                        .map_err(|error| HostCallError::new(error.to_string())),
                    Err(error) => Ok(crate::result_error(error.to_string())),
                }
            }),
            HostFunction::new(
                "encode",
                vec![HostParam::new("value", value_shape())],
                string_result_shape(),
            )
            .task_safe(true)
            .with_executor(|args: &[Value]| {
                let value = value_arg(args, 0, "value")?;
                let json = tune_to_json(value)?;
                match serde_json::to_string(&json) {
                    Ok(text) => Ok(crate::result_ok(Value::String(text))),
                    Err(error) => Ok(crate::result_error(error.to_string())),
                }
            }),
            HostFunction::new(
                "compact",
                vec![HostParam::new("text", Shape::String)],
                string_result_shape(),
            )
            .task_safe(true)
            .with_executor(|args: &[Value]| {
                let text = crate::string_arg(args, 0, "text")?;
                transform_json_text(text, serde_json::to_string)
            }),
            HostFunction::new(
                "pretty",
                vec![HostParam::new("text", Shape::String)],
                string_result_shape(),
            )
            .task_safe(true)
            .with_executor(|args: &[Value]| {
                let text = crate::string_arg(args, 0, "text")?;
                transform_json_text(text, serde_json::to_string_pretty)
            }),
            HostFunction::new("null", Vec::new(), value_shape())
                .task_safe(true)
                .with_executor(|_: &[Value]| Ok(json_null())),
            HostFunction::new(
                "bool",
                vec![HostParam::new("value", Shape::Bool)],
                value_shape(),
            )
            .task_safe(true)
            .with_executor(|args: &[Value]| {
                let value = bool_arg(args, 0, "value")?;
                Ok(json_bool(value))
            }),
            HostFunction::new(
                "number",
                vec![HostParam::new("value", Shape::Float)],
                value_shape(),
            )
            .task_safe(true)
            .with_executor(|args: &[Value]| {
                let value = float_arg(args, 0, "value")?;
                Ok(json_number(value))
            }),
            HostFunction::new(
                "string",
                vec![HostParam::new("value", Shape::String)],
                value_shape(),
            )
            .task_safe(true)
            .with_executor(|args: &[Value]| {
                let value = crate::string_arg(args, 0, "value")?;
                Ok(json_string(value))
            }),
            HostFunction::new(
                "array",
                vec![HostParam::new(
                    "items",
                    Shape::Sequence(Box::new(value_shape())),
                )],
                value_shape(),
            )
            .task_safe(true)
            .with_executor(|args: &[Value]| {
                let items = value_sequence_arg(args, 0, "items")?;
                Ok(json_array(items.to_vec()))
            }),
            HostFunction::new(
                "field",
                vec![
                    HostParam::new("name", Shape::String),
                    HostParam::new("value", value_shape()),
                ],
                field_shape(),
            )
            .task_safe(true)
            .with_executor(|args: &[Value]| {
                let name = crate::string_arg(args, 0, "name")?;
                let value = value_arg(args, 1, "value")?;
                Ok(json_field(name, value.clone()))
            }),
            HostFunction::new(
                "object",
                vec![HostParam::new(
                    "fields",
                    Shape::Sequence(Box::new(field_shape())),
                )],
                value_shape(),
            )
            .task_safe(true)
            .with_executor(|args: &[Value]| {
                let fields = field_sequence_arg(args, 0, "fields")?;
                Ok(json_object(fields.to_vec()))
            }),
        ],
    )
    .with_values(vec![
        HostValueType::new(
            "Value",
            vec![
                HostValueField::new("kind", Shape::String),
                HostValueField::new("bool", Shape::Optional(Box::new(Shape::Bool))),
                HostValueField::new("number", Shape::Optional(Box::new(Shape::Float))),
                HostValueField::new("string", Shape::Optional(Box::new(Shape::String))),
                HostValueField::new(
                    "items",
                    Shape::Optional(Box::new(Shape::Sequence(Box::new(value_shape())))),
                ),
                HostValueField::new(
                    "fields",
                    Shape::Optional(Box::new(Shape::Sequence(Box::new(field_shape())))),
                ),
            ],
        ),
        HostValueType::new(
            "Field",
            vec![
                HostValueField::new("name", Shape::String),
                HostValueField::new("value", value_shape()),
            ],
        ),
    ])
}

fn transform_json_text(
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

fn json_to_tune(value: &serde_json::Value) -> Result<Value, JsonNumberError> {
    match value {
        serde_json::Value::Null => Ok(json_null()),
        serde_json::Value::Bool(value) => Ok(json_bool(*value)),
        serde_json::Value::Number(value) => {
            let Some(value) = value.as_f64() else {
                return Err(JsonNumberError);
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

fn tune_to_json(value: &Value) -> Result<serde_json::Value, HostCallError> {
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

fn json_null() -> Value {
    json_value(
        "null",
        Value::None,
        Value::None,
        Value::None,
        Value::None,
        Value::None,
    )
}

fn json_bool(value: bool) -> Value {
    json_value(
        "bool",
        Value::Bool(value),
        Value::None,
        Value::None,
        Value::None,
        Value::None,
    )
}

fn json_number(value: f64) -> Value {
    json_value(
        "number",
        Value::None,
        Value::Float(value),
        Value::None,
        Value::None,
        Value::None,
    )
}

fn json_string(value: &str) -> Value {
    json_value(
        "string",
        Value::None,
        Value::None,
        Value::String(value.to_owned()),
        Value::None,
        Value::None,
    )
}

fn json_array(items: Vec<Value>) -> Value {
    json_value(
        "array",
        Value::None,
        Value::None,
        Value::None,
        Value::Sequence(items),
        Value::None,
    )
}

fn json_object(fields: Vec<Value>) -> Value {
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

fn json_field(name: &str, value: Value) -> Value {
    Value::HostStruct {
        type_name: FIELD_TYPE.into(),
        fields: vec![
            ("name".into(), Value::String(name.into())),
            ("value".into(), value),
        ],
    }
}

fn value_shape() -> Shape {
    Shape::Struct(VALUE_TYPE.into())
}

fn field_shape() -> Shape {
    Shape::Struct(FIELD_TYPE.into())
}

fn result_shape(ok: Shape) -> Shape {
    Shape::Result {
        ok: Box::new(ok),
        err: Box::new(Shape::String),
    }
}

fn string_result_shape() -> Shape {
    result_shape(Shape::String)
}

fn value_arg<'a>(args: &'a [Value], index: usize, name: &str) -> Result<&'a Value, HostCallError> {
    let Some(value) = args.get(index) else {
        return Err(HostCallError::new(format!(
            "missing argument `{name}` at index {index}"
        )));
    };
    host_struct_fields(value, VALUE_TYPE, name)?;
    Ok(value)
}

fn value_sequence_arg<'a>(
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

fn field_sequence_arg<'a>(
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

fn bool_arg(args: &[Value], index: usize, name: &str) -> Result<bool, HostCallError> {
    match args.get(index) {
        Some(Value::Bool(value)) => Ok(*value),
        None => Err(HostCallError::new(format!(
            "missing argument `{name}` at index {index}"
        ))),
        _ => Err(HostCallError::new(format!("expected Bool for `{name}`"))),
    }
}

fn float_arg(args: &[Value], index: usize, name: &str) -> Result<f64, HostCallError> {
    match args.get(index) {
        Some(Value::Float(value)) => Ok(*value),
        None => Err(HostCallError::new(format!(
            "missing argument `{name}` at index {index}"
        ))),
        _ => Err(HostCallError::new(format!("expected Float for `{name}`"))),
    }
}

fn host_struct_fields<'a>(
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

fn required_value_field<'a>(
    fields: &'a [(String, Value)],
    name: &str,
) -> Result<&'a Value, HostCallError> {
    fields
        .iter()
        .find(|(field_name, _)| field_name == name)
        .map(|(_, value)| value)
        .ok_or_else(|| HostCallError::new(format!("missing json field `{name}`")))
}

fn required_string_field(fields: &[(String, Value)], name: &str) -> Result<String, HostCallError> {
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

#[derive(Debug, Clone, Copy)]
struct JsonNumberError;

impl std::fmt::Display for JsonNumberError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("JSON number cannot be represented as Float")
    }
}
