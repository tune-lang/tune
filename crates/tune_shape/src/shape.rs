#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ShapeId(pub u32);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Shape {
    Hole,
    Never,
    Unit,
    Int,
    Float,
    Size,
    Byte,
    Bool,
    String,
    Sequence(Box<Shape>),
    Tuple(Vec<Shape>),
    Union(Vec<Shape>),
    Optional(Box<Shape>),
    Callable { params: Vec<Shape>, ret: Box<Shape> },
    Result { ok: Box<Shape>, err: Box<Shape> },
    Task(Box<Shape>),
    Struct(String),
    Enum(String),
    Structural(Vec<MemberRequirement>),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MemberRequirement {
    Field {
        name: String,
        shape: Option<Shape>,
    },
    Callable {
        name: String,
        params: Vec<Shape>,
        ret: Option<Shape>,
    },
}
