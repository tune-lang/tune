use crate::shape::Shape;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum LiteralFact {
    Numeric { text: String },
    String { segments: Vec<String> },
    Sequence { elements: Vec<LiteralFact> },
    Bool(bool),
    None,
    Unit,
}

impl LiteralFact {
    #[must_use]
    pub const fn is_numeric(&self) -> bool {
        matches!(self, Self::Numeric { .. })
    }

    #[must_use]
    pub fn storage_shape(&self) -> Shape {
        match self {
            Self::Numeric { .. } => Shape::Int,
            Self::String { .. } => Shape::String,
            Self::Sequence { elements } => Shape::Sequence(Box::new(Shape::join_all(
                elements.iter().map(Self::storage_shape),
            ))),
            Self::Bool(_) => Shape::Bool,
            Self::None => Shape::Optional(Box::new(Shape::Hole)),
            Self::Unit => Shape::Unit,
        }
    }

    pub fn default_for(shape: &Shape) -> Option<Self> {
        match shape {
            Shape::Int | Shape::Size | Shape::Byte => Some(Self::Numeric { text: "0".into() }),
            Shape::Float => Some(Self::Numeric { text: "0.0".into() }),
            Shape::Bool => Some(Self::Bool(false)),
            Shape::String => Some(Self::String {
                segments: vec![String::new()],
            }),
            Shape::Unit => Some(Self::Unit),
            Shape::Optional(_) => Some(Self::None),
            Shape::Sequence(_) => Some(Self::Sequence {
                elements: Vec::new(),
            }),
            Shape::Literal(fact) => Some(fact.clone()),
            _ => None,
        }
    }
}
