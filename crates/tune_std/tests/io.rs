#[test]
fn io_module_exposes_result_io_functions() -> Result<(), &'static str> {
    let module = tune_std::io::install();

    for name in ["write", "write_line", "error_line", "read_line"] {
        let function = module
            .functions
            .iter()
            .find(|function| function.name == name)
            .ok_or("io function should be installed")?;
        assert!(matches!(function.ret, tune_shape::Shape::Result { .. }));
        assert!(!function.task_safe);
        assert!(function.executor.is_some());
    }

    let write = module
        .functions
        .iter()
        .find(|function| function.name == "write")
        .ok_or("io.write should be installed")?;
    assert!(
        write
            .authorities
            .iter()
            .any(|authority| authority.0 == "io.write")
    );

    let error_line = module
        .functions
        .iter()
        .find(|function| function.name == "error_line")
        .ok_or("io.error_line should be installed")?;
    assert!(
        error_line
            .authorities
            .iter()
            .any(|authority| authority.0 == "io.error")
    );

    let read_line = module
        .functions
        .iter()
        .find(|function| function.name == "read_line")
        .ok_or("io.read_line should be installed")?;
    assert!(
        read_line
            .authorities
            .iter()
            .any(|authority| authority.0 == "io.read")
    );

    Ok(())
}
