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
    pub fn accepts(&self, value: &Self) -> bool {
        match (self, value) {
            (Self::Hole, _) | (_, Self::Hole) => true,
            (_, Self::Never) => true,
            (expected, actual) if expected == actual => true,
            (Self::Union(items), actual) => items.iter().any(|item| item.accepts(actual)),
            (expected, Self::Union(items)) => items.iter().all(|item| expected.accepts(item)),
            (Self::Optional(inner), Self::Optional(actual)) => inner.accepts(actual),
            (Self::Optional(_), Self::Literal(crate::literal::LiteralFact::None)) => true,
            (Self::Sequence(expected), Self::Sequence(actual)) => expected.accepts(actual),
            (Self::Tuple(expected), Self::Tuple(actual)) if expected.len() == actual.len() => {
                expected
                    .iter()
                    .zip(actual)
                    .all(|(expected, actual)| expected.accepts(actual))
            }
            (
                Self::Callable { params, ret },
                Self::Callable {
                    params: actual_params,
                    ret: actual_ret,
                },
            ) if params.len() == actual_params.len() => {
                params
                    .iter()
                    .zip(actual_params)
                    .all(|(expected, actual)| expected.accepts(actual))
                    && ret.accepts(actual_ret)
            }
            (
                Self::Result { ok, err },
                Self::Result {
                    ok: actual_ok,
                    err: actual_err,
                },
            ) => ok.accepts(actual_ok) && err.accepts(actual_err),
            (Self::Task(expected), Self::Task(actual)) => expected.accepts(actual),
            (
                Self::Apply { name, args },
                Self::Apply {
                    name: actual_name,
                    args: actual_args,
                },
            ) if name == actual_name && args.len() == actual_args.len() => args
                .iter()
                .zip(actual_args)
                .all(|(expected, actual)| expected.accepts(actual)),
            (expected, Self::Literal(literal)) => crate::can_materialize(literal, expected),
            _ => false,
        }
    }

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
