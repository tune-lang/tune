fn text_executor<'a>(
    module: &'a tune_host::HostModule,
    name: &str,
) -> Result<&'a tune_host::HostExecutor, &'static str> {
    module
        .functions
        .iter()
        .find(|function| function.name == name)
        .and_then(|function| function.executor.as_ref())
        .ok_or("text function should carry an executor")
}

#[test]
fn text_bytes_executor_returns_byte_sequence() -> Result<(), &'static str> {
    let module = tune_std::text::install();
    let value = text_executor(&module, "bytes")?
        .call(&[tune_runtime::Value::String("Tune".into())])
        .map_err(|_| "text.bytes should execute")?;

    assert_eq!(
        value,
        tune_runtime::Value::Sequence(vec![
            tune_runtime::Value::Byte(b'T'),
            tune_runtime::Value::Byte(b'u'),
            tune_runtime::Value::Byte(b'n'),
            tune_runtime::Value::Byte(b'e'),
        ])
    );

    Ok(())
}

#[test]
fn text_length_executors_distinguish_bytes_and_chars() -> Result<(), &'static str> {
    let module = tune_std::text::install();

    assert_eq!(
        text_executor(&module, "byte_len")?
            .call(&[tune_runtime::Value::String("é".into())])
            .map_err(|_| "text.byte_len should execute")?,
        tune_runtime::Value::Size(2)
    );
    assert_eq!(
        text_executor(&module, "char_count")?
            .call(&[tune_runtime::Value::String("é".into())])
            .map_err(|_| "text.char_count should execute")?,
        tune_runtime::Value::Size(1)
    );

    Ok(())
}

#[test]
fn text_from_utf8_executor_returns_result_values() -> Result<(), &'static str> {
    let module = tune_std::text::install();
    let value = text_executor(&module, "from_utf8")?
        .call(&[tune_runtime::Value::Sequence(vec![
            tune_runtime::Value::Byte(b'o'),
            tune_runtime::Value::Byte(b'k'),
        ])])
        .map_err(|_| "text.from_utf8 should execute")?;

    assert_eq!(
        value,
        tune_runtime::Value::Variant {
            variant: tune_runtime::value::RuntimeVariant::ResultOk,
            fields: vec![tune_runtime::Value::String("ok".into())],
            propagation_frames: Vec::new(),
        }
    );

    let invalid = text_executor(&module, "from_utf8")?
        .call(&[tune_runtime::Value::Sequence(vec![
            tune_runtime::Value::Byte(0xff),
        ])])
        .map_err(|_| "text.from_utf8 should execute")?;
    assert!(matches!(
        invalid,
        tune_runtime::Value::Variant {
            variant: tune_runtime::value::RuntimeVariant::ResultError,
            ..
        }
    ));

    Ok(())
}

#[test]
fn text_transform_executors_return_strings() -> Result<(), &'static str> {
    let module = tune_std::text::install();

    assert_eq!(
        text_executor(&module, "trim")?
            .call(&[tune_runtime::Value::String("  Tune  ".into())])
            .map_err(|_| "text.trim should execute")?,
        tune_runtime::Value::String("Tune".into())
    );
    assert_eq!(
        text_executor(&module, "lower")?
            .call(&[tune_runtime::Value::String("Tune".into())])
            .map_err(|_| "text.lower should execute")?,
        tune_runtime::Value::String("tune".into())
    );
    assert_eq!(
        text_executor(&module, "upper")?
            .call(&[tune_runtime::Value::String("Tune".into())])
            .map_err(|_| "text.upper should execute")?,
        tune_runtime::Value::String("TUNE".into())
    );
    assert_eq!(
        text_executor(&module, "replace")?
            .call(&[
                tune_runtime::Value::String("hello tune".into()),
                tune_runtime::Value::String("tune".into()),
                tune_runtime::Value::String("dyno".into()),
            ])
            .map_err(|_| "text.replace should execute")?,
        tune_runtime::Value::String("hello dyno".into())
    );

    Ok(())
}

#[test]
fn text_split_executors_return_string_sequences() -> Result<(), &'static str> {
    let module = tune_std::text::install();

    assert_eq!(
        text_executor(&module, "split")?
            .call(&[
                tune_runtime::Value::String("a,b,c".into()),
                tune_runtime::Value::String(",".into()),
            ])
            .map_err(|_| "text.split should execute")?,
        tune_runtime::Value::Sequence(vec![
            tune_runtime::Value::String("a".into()),
            tune_runtime::Value::String("b".into()),
            tune_runtime::Value::String("c".into()),
        ])
    );
    assert_eq!(
        text_executor(&module, "lines")?
            .call(&[tune_runtime::Value::String("one\ntwo".into())])
            .map_err(|_| "text.lines should execute")?,
        tune_runtime::Value::Sequence(vec![
            tune_runtime::Value::String("one".into()),
            tune_runtime::Value::String("two".into()),
        ])
    );
    assert_eq!(
        text_executor(&module, "join")?
            .call(&[
                tune_runtime::Value::Sequence(vec![
                    tune_runtime::Value::String("a".into()),
                    tune_runtime::Value::String("b".into()),
                    tune_runtime::Value::String("c".into()),
                ]),
                tune_runtime::Value::String("-".into()),
            ])
            .map_err(|_| "text.join should execute")?,
        tune_runtime::Value::String("a-b-c".into())
    );

    Ok(())
}

#[test]
fn text_slice_executor_returns_result_values() -> Result<(), &'static str> {
    let module = tune_std::text::install();

    assert_eq!(
        text_executor(&module, "slice")?
            .call(&[
                tune_runtime::Value::String("Tune".into()),
                tune_runtime::Value::Size(1),
                tune_runtime::Value::Size(2),
            ])
            .map_err(|_| "text.slice should execute")?,
        tune_runtime::Value::Variant {
            variant: tune_runtime::value::RuntimeVariant::ResultOk,
            fields: vec![tune_runtime::Value::String("un".into())],
            propagation_frames: Vec::new(),
        }
    );

    let out_of_bounds = text_executor(&module, "slice")?
        .call(&[
            tune_runtime::Value::String("Tune".into()),
            tune_runtime::Value::Size(3),
            tune_runtime::Value::Size(2),
        ])
        .map_err(|_| "text.slice should execute")?;
    assert!(matches!(
        out_of_bounds,
        tune_runtime::Value::Variant {
            variant: tune_runtime::value::RuntimeVariant::ResultError,
            ..
        }
    ));

    Ok(())
}
