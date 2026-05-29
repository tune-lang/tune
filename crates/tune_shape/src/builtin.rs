use crate::{NominalShape, Shape};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BuiltinShape {
    Never,
    Unit,
    Int,
    Float,
    Size,
    Byte,
    Bool,
    String,
    Result,
    Task,
    Map,
    Set,
}

impl BuiltinShape {
    #[must_use]
    pub fn named(name: &str) -> Option<Self> {
        match name {
            "Never" => Some(Self::Never),
            "()" | "Unit" => Some(Self::Unit),
            "Int" => Some(Self::Int),
            "Float" => Some(Self::Float),
            "Size" => Some(Self::Size),
            "Byte" => Some(Self::Byte),
            "Bool" => Some(Self::Bool),
            "String" => Some(Self::String),
            "Result" => Some(Self::Result),
            "Task" => Some(Self::Task),
            "Map" => Some(Self::Map),
            "Set" => Some(Self::Set),
            _ => None,
        }
    }

    #[must_use]
    pub const fn arity(self) -> usize {
        match self {
            Self::Result | Self::Map => 2,
            Self::Task | Self::Set => 1,
            Self::Never
            | Self::Unit
            | Self::Int
            | Self::Float
            | Self::Size
            | Self::Byte
            | Self::Bool
            | Self::String => 0,
        }
    }

    #[must_use]
    pub fn bare_shape(self) -> Shape {
        let args = vec![Shape::Hole; self.arity()];
        self.apply(args).unwrap_or(Shape::Hole)
    }

    #[must_use]
    pub fn apply(self, args: Vec<Shape>) -> Option<Shape> {
        if args.len() != self.arity() {
            return None;
        }
        match self {
            Self::Never => Some(Shape::Never),
            Self::Unit => Some(Shape::Unit),
            Self::Int => Some(Shape::Int),
            Self::Float => Some(Shape::Float),
            Self::Size => Some(Shape::Size),
            Self::Byte => Some(Shape::Byte),
            Self::Bool => Some(Shape::Bool),
            Self::String => Some(Shape::String),
            Self::Result => Some(Shape::Result {
                ok: Box::new(args[0].clone()),
                err: Box::new(args[1].clone()),
            }),
            Self::Task => Some(Shape::Task(Box::new(args[0].clone()))),
            Self::Map => Some(Shape::Apply {
                nominal: NominalShape::external("Map"),
                args,
            }),
            Self::Set => Some(Shape::Apply {
                nominal: NominalShape::external("Set"),
                args,
            }),
        }
    }
}

#[must_use]
pub fn builtin_shape(name: &str) -> Option<Shape> {
    BuiltinShape::named(name).map(BuiltinShape::bare_shape)
}

#[must_use]
pub fn builtin_generic_shape(name: &str, args: Vec<Shape>) -> Option<Shape> {
    BuiltinShape::named(name)?.apply(args)
}
