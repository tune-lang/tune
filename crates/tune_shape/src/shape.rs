use tune_diagnostics::Span;
use tune_hir::HirId;

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

    pub fn alloc(&mut self, shape: Shape, origin: ShapeOrigin) -> Option<ShapeId> {
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
pub struct NominalShape {
    pub id: Option<HirId>,
    pub name: String,
}

impl NominalShape {
    #[must_use]
    pub fn new(id: HirId, name: impl Into<String>) -> Self {
        Self {
            id: Some(id),
            name: name.into(),
        }
    }

    #[must_use]
    pub fn external(name: impl Into<String>) -> Self {
        Self {
            id: None,
            name: name.into(),
        }
    }

    #[must_use]
    pub fn same_identity(&self, other: &Self) -> bool {
        match (self.id, other.id) {
            (Some(left), Some(right)) => left == right,
            _ => self.name == other.name,
        }
    }
}

impl From<&str> for NominalShape {
    fn from(value: &str) -> Self {
        Self::external(value)
    }
}

impl From<String> for NominalShape {
    fn from(value: String) -> Self {
        Self::external(value)
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
    Range(Box<Shape>),
    Tuple(Vec<Shape>),
    Union(Vec<Shape>),
    Optional(Box<Shape>),
    Callable {
        params: Vec<Shape>,
        ret: Box<Shape>,
    },
    Result {
        ok: Box<Shape>,
        err: Box<Shape>,
    },
    Task(Box<Shape>),
    Apply {
        nominal: NominalShape,
        args: Vec<Shape>,
    },
    Struct(NominalShape),
    Enum(NominalShape),
    Structural(Vec<MemberRequirement>),
}

impl Shape {
    #[must_use]
    pub fn product(items: Vec<Self>) -> Self {
        match items.as_slice() {
            [] => Self::Unit,
            [item] => item.clone(),
            _ => Self::Tuple(items),
        }
    }

    #[must_use]
    pub const fn nominal(&self) -> Option<&NominalShape> {
        match self {
            Self::Struct(nominal) | Self::Enum(nominal) | Self::Apply { nominal, .. } => {
                Some(nominal)
            }
            _ => None,
        }
    }

    #[must_use]
    pub fn nominal_name(&self) -> Option<&str> {
        self.nominal().map(|nominal| nominal.name.as_str())
    }

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
            (Self::Optional(inner), actual) => inner.accepts(actual),
            (Self::Sequence(expected), Self::Sequence(actual)) => expected.accepts(actual),
            (Self::Range(expected), Self::Range(actual)) => expected.accepts(actual),
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
                Self::Apply { nominal, args },
                Self::Apply {
                    nominal: actual_nominal,
                    args: actual_args,
                },
            ) if nominal.same_identity(actual_nominal) && args.len() == actual_args.len() => args
                .iter()
                .zip(actual_args)
                .all(|(expected, actual)| expected.accepts(actual)),
            (Self::Structural(requirements), actual) => actual.satisfies_requirements(requirements),
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

    fn satisfies_requirements(&self, requirements: &[MemberRequirement]) -> bool {
        let Self::Structural(actual) = self else {
            return matches!(self, Self::Hole);
        };

        requirements
            .iter()
            .all(|requirement| requirement_satisfied(requirement, actual))
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

fn requirement_satisfied(expected: &MemberRequirement, actual: &[MemberRequirement]) -> bool {
    actual
        .iter()
        .any(|actual| member_requirement_accepts(expected, actual))
}

fn member_requirement_accepts(expected: &MemberRequirement, actual: &MemberRequirement) -> bool {
    match (expected, actual) {
        (
            MemberRequirement::Field { name, shape },
            MemberRequirement::Field {
                name: actual_name,
                shape: actual_shape,
            },
        ) if name == actual_name => optional_shape_accepts(shape.as_ref(), actual_shape.as_ref()),
        (
            MemberRequirement::Callable { name, params, ret },
            MemberRequirement::Callable {
                name: actual_name,
                params: actual_params,
                ret: actual_ret,
            },
        ) if name == actual_name && params.len() == actual_params.len() => {
            params
                .iter()
                .zip(actual_params)
                .all(|(expected, actual)| expected.accepts(actual))
                && optional_shape_accepts(ret.as_ref(), actual_ret.as_ref())
        }
        _ => false,
    }
}

fn optional_shape_accepts(expected: Option<&Shape>, actual: Option<&Shape>) -> bool {
    match (expected, actual) {
        (None, _) | (_, None) => true,
        (Some(expected), Some(actual)) => expected.accepts(actual),
    }
}
