use crate::{LiteralFact, Shape};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Commitment {
    PerUse,
    CommitBinding,
}

#[derive(Debug, Clone)]
pub struct MaterializationPlan {
    pub target: Shape,
    pub commitment: Commitment,
}

pub fn can_materialize(lit: &LiteralFact, target: &Shape) -> bool {
    matches!((lit, target), (_, Shape::Hole)) || !matches!(target, Shape::Never)
}
