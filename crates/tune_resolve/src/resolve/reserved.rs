pub(super) fn is_stdcore_name(name: &str) -> bool {
    matches!(
        name,
        "Never"
            | "Unit"
            | "Int"
            | "Float"
            | "Size"
            | "Byte"
            | "Bool"
            | "String"
            | "Result"
            | "Task"
            | "Map"
            | "Set"
    )
}
