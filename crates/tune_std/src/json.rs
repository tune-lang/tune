mod value;

use self::value::{
    VALUE_TYPE, bool_arg, field_arg, field_sequence_arg, field_shape, float_arg,
    host_struct_fields, json_array, json_bool, json_field, json_null, json_number, json_object,
    json_string, json_to_tune, optional_json_field, required_string_field, required_value_field,
    result_shape, string_result_shape, transform_json_text, tune_to_json, value_arg,
    value_sequence_arg, value_shape,
};
use tune_host::{HostFunction, HostModule, HostParam, HostValueField, HostValueType};
use tune_runtime::Value;
use tune_shape::Shape;

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
                    Ok(value) => json_to_tune(&value).map(crate::result_ok),
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
            HostFunction::new(
                "kind",
                vec![HostParam::new("value", value_shape())],
                Shape::String,
            )
            .task_safe(true)
            .with_executor(|args: &[Value]| {
                let value = value_arg(args, 0, "value")?;
                let fields = host_struct_fields(value, VALUE_TYPE, "value")?;
                Ok(Value::String(required_string_field(fields, "kind")?))
            }),
            HostFunction::new(
                "as_bool",
                vec![HostParam::new("value", value_shape())],
                Shape::Optional(Box::new(Shape::Bool)),
            )
            .task_safe(true)
            .with_executor(|args: &[Value]| {
                let value = value_arg(args, 0, "value")?;
                let fields = host_struct_fields(value, VALUE_TYPE, "value")?;
                optional_json_field(fields, "bool")
            }),
            HostFunction::new(
                "as_number",
                vec![HostParam::new("value", value_shape())],
                Shape::Optional(Box::new(Shape::Float)),
            )
            .task_safe(true)
            .with_executor(|args: &[Value]| {
                let value = value_arg(args, 0, "value")?;
                let fields = host_struct_fields(value, VALUE_TYPE, "value")?;
                optional_json_field(fields, "number")
            }),
            HostFunction::new(
                "as_string",
                vec![HostParam::new("value", value_shape())],
                Shape::Optional(Box::new(Shape::String)),
            )
            .task_safe(true)
            .with_executor(|args: &[Value]| {
                let value = value_arg(args, 0, "value")?;
                let fields = host_struct_fields(value, VALUE_TYPE, "value")?;
                optional_json_field(fields, "string")
            }),
            HostFunction::new(
                "items",
                vec![HostParam::new("value", value_shape())],
                Shape::Optional(Box::new(Shape::Sequence(Box::new(value_shape())))),
            )
            .task_safe(true)
            .with_executor(|args: &[Value]| {
                let value = value_arg(args, 0, "value")?;
                let fields = host_struct_fields(value, VALUE_TYPE, "value")?;
                optional_json_field(fields, "items")
            }),
            HostFunction::new(
                "fields",
                vec![HostParam::new("value", value_shape())],
                Shape::Optional(Box::new(Shape::Sequence(Box::new(field_shape())))),
            )
            .task_safe(true)
            .with_executor(|args: &[Value]| {
                let value = value_arg(args, 0, "value")?;
                let fields = host_struct_fields(value, VALUE_TYPE, "value")?;
                optional_json_field(fields, "fields")
            }),
            HostFunction::new(
                "field_name",
                vec![HostParam::new("field", field_shape())],
                Shape::String,
            )
            .task_safe(true)
            .with_executor(|args: &[Value]| {
                let fields = field_arg(args, 0, "field")?;
                Ok(Value::String(required_string_field(fields, "name")?))
            }),
            HostFunction::new(
                "field_value",
                vec![HostParam::new("field", field_shape())],
                value_shape(),
            )
            .task_safe(true)
            .with_executor(|args: &[Value]| {
                let fields = field_arg(args, 0, "field")?;
                Ok(required_value_field(fields, "value")?.clone())
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
