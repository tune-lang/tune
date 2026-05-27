#[test]
fn smoke() {
    let crate_name = env!("CARGO_PKG_NAME");
    assert!(!crate_name.is_empty());
}

#[test]
fn core_opcodes_reserve_dense_bytecode_slots() -> Result<(), &'static str> {
    assert_eq!(tune_bytecode::Opcode::ALL.len(), 33);
    for (index, opcode) in tune_bytecode::Opcode::ALL.iter().enumerate() {
        let expected = u8::try_from(index).map_err(|_| "opcode index overflow")?;
        assert_eq!(*opcode as u8, expected);
    }

    assert!(tune_bytecode::Opcode::ALL.contains(&tune_bytecode::Opcode::SeqSetChecked));
    assert!(tune_bytecode::Opcode::ALL.contains(&tune_bytecode::Opcode::VariantConstruct));
    assert!(tune_bytecode::Opcode::ALL.contains(&tune_bytecode::Opcode::MatchVariant));
    assert!(tune_bytecode::Opcode::ALL.contains(&tune_bytecode::Opcode::FiniteForInit));
    assert!(tune_bytecode::Opcode::ALL.contains(&tune_bytecode::Opcode::ResultPropagate));
    assert!(tune_bytecode::Opcode::ALL.contains(&tune_bytecode::Opcode::SpawnTask));

    Ok(())
}
