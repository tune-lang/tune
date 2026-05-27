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
