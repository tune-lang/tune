#[derive(Debug, Clone)]
pub enum PatternKind {
    Hole,
    Binding(String),
    Unit,
    Tuple(Vec<PatternKind>),
    Variant {
        name: String,
        args: Vec<PatternKind>,
    },
    StructuralShape,
    Else,
}
