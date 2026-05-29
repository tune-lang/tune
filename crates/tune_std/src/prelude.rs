#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StdCoreShape {
    Result,
    Map,
    Set,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StdCoreFunction {
    Print,
    Some,
    None,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StdCoreRegistry {
    pub shapes: Vec<StdCoreShape>,
    pub functions: Vec<StdCoreFunction>,
}

#[must_use]
pub fn stdcore() -> StdCoreRegistry {
    StdCoreRegistry {
        shapes: vec![StdCoreShape::Result, StdCoreShape::Map, StdCoreShape::Set],
        functions: vec![
            StdCoreFunction::Print,
            StdCoreFunction::Some,
            StdCoreFunction::None,
        ],
    }
}

#[must_use]
pub fn install() -> StdCoreRegistry {
    stdcore()
}
