fn executor<'a>(
    module: &'a tune_host::HostModule,
    name: &str,
) -> Result<&'a tune_host::HostExecutor, &'static str> {
    module
        .functions
        .iter()
        .find(|function| function.name == name)
        .and_then(|function| function.executor.as_ref())
        .ok_or("json function should carry an executor")
}

fn result_ok(value: tune_runtime::Value) -> Result<tune_runtime::Value, &'static str> {
    let tune_runtime::Value::Variant {
        variant: tune_runtime::value::RuntimeVariant::ResultOk,
        fields,
        ..
    } = value
    else {
        return Err("expected Result Ok");
    };
    fields.into_iter().next().ok_or("Ok should carry payload")
}

fn field<'a>(
    fields: &'a [(String, tune_runtime::Value)],
    name: &str,
) -> Result<&'a tune_runtime::Value, &'static str> {
    fields
        .iter()
        .find(|(field_name, _)| field_name == name)
        .map(|(_, value)| value)
        .ok_or("host struct field should exist")
}

#[test]
fn json_module_exposes_value_types_and_task_safe_functions() -> Result<(), &'static str> {
    let module = tune_std::json::install();

    assert_eq!(module.name, "json");
    assert!(module.values.iter().any(|value| value.name == "Value"));
    assert!(module.values.iter().any(|value| value.name == "Field"));

    for name in [
        "valid",
        "decode",
        "encode",
        "compact",
        "pretty",
        "null",
        "bool",
        "number",
        "string",
        "array",
        "field",
        "object",
        "kind",
        "as_bool",
        "as_number",
        "as_string",
        "items",
        "fields",
        "field_name",
        "field_value",
    ] {
        let function = module
            .functions
            .iter()
            .find(|function| function.name == name)
            .ok_or("json function should be installed")?;
        assert!(function.task_safe);
        assert!(function.authorities.is_empty());
    }

    Ok(())
}

#[test]
fn json_text_helpers_validate_and_format() -> Result<(), &'static str> {
    let module = tune_std::json::install();

    assert_eq!(
        executor(&module, "valid")?
            .call(&[tune_runtime::Value::String("{\"ok\":true}".into())])
            .map_err(|_| "json.valid should execute")?,
        tune_runtime::Value::Bool(true)
    );
    assert_eq!(
        executor(&module, "valid")?
            .call(&[tune_runtime::Value::String("{bad".into())])
            .map_err(|_| "json.valid should execute")?,
        tune_runtime::Value::Bool(false)
    );

    let compact = result_ok(
        executor(&module, "compact")?
            .call(&[tune_runtime::Value::String("{ \"ok\" : true }".into())])
            .map_err(|_| "json.compact should execute")?,
    )?;
    assert_eq!(compact, tune_runtime::Value::String("{\"ok\":true}".into()));

    let pretty = result_ok(
        executor(&module, "pretty")?
            .call(&[tune_runtime::Value::String("{\"ok\":true}".into())])
            .map_err(|_| "json.pretty should execute")?,
    )?;
    let tune_runtime::Value::String(pretty) = pretty else {
        return Err("json.pretty should return string payload");
    };
    assert!(pretty.contains('\n'));
    assert!(pretty.contains("\"ok\""));

    Ok(())
}

#[test]
fn json_decode_returns_host_value_tree() -> Result<(), &'static str> {
    let module = tune_std::json::install();
    let decoded = result_ok(
        executor(&module, "decode")?
            .call(&[tune_runtime::Value::String(
                "{\"name\":\"Tune\",\"items\":[true,null,2.5]}".into(),
            )])
            .map_err(|_| "json.decode should execute")?,
    )?;

    let tune_runtime::Value::HostStruct { type_name, fields } = decoded else {
        return Err("json.decode should return json.Value");
    };
    assert_eq!(type_name, "json.Value");
    assert_eq!(
        field(&fields, "kind")?,
        &tune_runtime::Value::String("object".into())
    );

    let tune_runtime::Value::Sequence(object_fields) = field(&fields, "fields")? else {
        return Err("object fields should be a sequence");
    };
    assert_eq!(object_fields.len(), 2);

    Ok(())
}

#[test]
fn json_accessors_read_value_and_field_parts() -> Result<(), &'static str> {
    let module = tune_std::json::install();
    let decoded = result_ok(
        executor(&module, "decode")?
            .call(&[tune_runtime::Value::String(
                "{\"name\":\"Tune\",\"ok\":true}".into(),
            )])
            .map_err(|_| "json.decode should execute")?,
    )?;

    assert_eq!(
        executor(&module, "kind")?
            .call(std::slice::from_ref(&decoded))
            .map_err(|_| "json.kind should execute")?,
        tune_runtime::Value::String("object".into())
    );
    let object_fields = executor(&module, "fields")?
        .call(std::slice::from_ref(&decoded))
        .map_err(|_| "json.fields should execute")?;
    let tune_runtime::Value::Sequence(fields) = object_fields else {
        return Err("json.fields should return field sequence for object");
    };
    let first = fields.first().ok_or("object should contain a field")?;
    let name = executor(&module, "field_name")?
        .call(std::slice::from_ref(first))
        .map_err(|_| "json.field_name should execute")?;
    assert!(matches!(name, tune_runtime::Value::String(_)));
    let value = executor(&module, "field_value")?
        .call(std::slice::from_ref(first))
        .map_err(|_| "json.field_value should execute")?;
    assert!(matches!(
        value,
        tune_runtime::Value::HostStruct { type_name, .. } if type_name == "json.Value"
    ));

    let string_value = executor(&module, "string")?
        .call(&[tune_runtime::Value::String("Tune".into())])
        .map_err(|_| "json.string should execute")?;
    assert_eq!(
        executor(&module, "as_string")?
            .call(&[string_value])
            .map_err(|_| "json.as_string should execute")?,
        tune_runtime::Value::String("Tune".into())
    );

    Ok(())
}

#[test]
fn json_encode_serializes_constructed_values() -> Result<(), &'static str> {
    let module = tune_std::json::install();
    let name = executor(&module, "string")?
        .call(&[tune_runtime::Value::String("Tune".into())])
        .map_err(|_| "json.string should execute")?;
    let count = executor(&module, "number")?
        .call(&[tune_runtime::Value::Float(2.0)])
        .map_err(|_| "json.number should execute")?;
    let name_field = executor(&module, "field")?
        .call(&[tune_runtime::Value::String("name".into()), name])
        .map_err(|_| "json.field should execute")?;
    let count_field = executor(&module, "field")?
        .call(&[tune_runtime::Value::String("count".into()), count])
        .map_err(|_| "json.field should execute")?;
    let object = executor(&module, "object")?
        .call(&[tune_runtime::Value::Sequence(vec![name_field, count_field])])
        .map_err(|_| "json.object should execute")?;
    let encoded = result_ok(
        executor(&module, "encode")?
            .call(&[object])
            .map_err(|_| "json.encode should execute")?,
    )?;
    let tune_runtime::Value::String(encoded) = encoded else {
        return Err("json.encode should return string payload");
    };
    let parsed: serde_json::Value =
        serde_json::from_str(&encoded).map_err(|_| "encoded JSON should parse")?;

    assert_eq!(parsed["name"], serde_json::Value::String("Tune".into()));
    assert_eq!(parsed["count"].as_f64(), Some(2.0));

    Ok(())
}
