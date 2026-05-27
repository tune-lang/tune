use tune_diagnostics::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ShapeId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShapeOrigin {
    Builtin,
    Annotation(Span),
    Inferred(Span),
    Synthetic,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShapeFact {
    pub id: ShapeId,
    pub shape: Shape,
    pub origin: ShapeOrigin,
}

#[derive(Debug, Default)]
pub struct ShapeStore {
    facts: Vec<ShapeFact>,
}

impl ShapeStore {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn intern(&mut self, shape: Shape, origin: ShapeOrigin) -> Option<ShapeId> {
        let index = u32::try_from(self.facts.len()).ok()?;
        let id = ShapeId(index);
        self.facts.push(ShapeFact { id, shape, origin });
        Some(id)
    }

    #[must_use]
    pub fn get(&self, id: ShapeId) -> Option<&ShapeFact> {
        self.facts.get(id.0 as usize).filter(|fact| fact.id == id)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.facts.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.facts.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &ShapeFact> {
        self.facts.iter()
    }
}

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
    Literal(crate::literal::LiteralFact),
    Param(String),
    Sequence(Box<Shape>),
    Tuple(Vec<Shape>),
    Union(Vec<Shape>),
    Optional(Box<Shape>),
    Callable { params: Vec<Shape>, ret: Box<Shape> },
    Result { ok: Box<Shape>, err: Box<Shape> },
    Task(Box<Shape>),
    Apply { name: String, args: Vec<Shape> },
    Struct(String),
    Enum(String),
    Structural(Vec<MemberRequirement>),
}

impl Shape {
    #[must_use]
    pub fn join(self, next: Self) -> Self {
        match (self, next) {
            (Self::Hole, shape) | (shape, Self::Hole) => shape,
            (Self::Union(items), Self::Union(next_items)) => {
                Self::join_all(items.into_iter().chain(next_items).collect::<Vec<_>>())
            }
            (Self::Union(items), shape) | (shape, Self::Union(items)) => {
                Self::join_all(items.into_iter().chain([shape]).collect::<Vec<_>>())
            }
            (existing, next) if existing == next => existing,
            (existing, next) => Self::Union(vec![existing, next]),
        }
    }

    #[must_use]
    pub fn join_all(shapes: impl IntoIterator<Item = Self>) -> Self {
        let mut unique = Vec::new();
        for shape in shapes {
            match shape {
                Self::Hole => {}
                Self::Union(items) => {
                    for item in items {
                        if item != Self::Hole && !unique.contains(&item) {
                            unique.push(item);
                        }
                    }
                }
                shape if !unique.contains(&shape) => unique.push(shape),
                _ => {}
            }
        }

        match unique.as_slice() {
            [] => Self::Hole,
            [shape] => shape.clone(),
            _ => Self::Union(unique),
        }
    }
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
