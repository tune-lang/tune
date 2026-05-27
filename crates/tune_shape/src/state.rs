use tune_diagnostics::Span;
use tune_hir::{HirId, MemberId};
use tune_resolve::LocalId;

use crate::{Commitment, LiteralFact, MaterializationPlan, Shape, can_materialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BindingKey {
    TopLevel(HirId),
    Param(MemberId),
    Local(LocalId),
    SelfValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindingState {
    pub key: BindingKey,
    pub name: Option<String>,
    pub storage_shape: Shape,
    pub current_shape: Shape,
    pub literal_fact: Option<LiteralFact>,
    pub materialization: Option<MaterializationPlan>,
    pub span: Option<Span>,
}

impl BindingState {
    #[must_use]
    pub fn new(
        key: BindingKey,
        name: Option<String>,
        storage_shape: Shape,
        current_shape: Shape,
        span: Option<Span>,
    ) -> Self {
        Self {
            key,
            name,
            storage_shape,
            current_shape,
            literal_fact: None,
            materialization: None,
            span,
        }
    }

    #[must_use]
    pub fn literal(
        key: BindingKey,
        name: Option<String>,
        storage_shape: Shape,
        literal_fact: LiteralFact,
        span: Option<Span>,
    ) -> Self {
        Self {
            key,
            name,
            storage_shape,
            current_shape: Shape::Literal(literal_fact.clone()),
            literal_fact: Some(literal_fact),
            materialization: None,
            span,
        }
    }

    pub fn narrow_current(&mut self, shape: Shape) {
        self.current_shape = shape;
        self.literal_fact = None;
        self.materialization = None;
    }

    pub fn assign_shape(&mut self, shape: Shape) {
        self.current_shape = shape;
        self.literal_fact = None;
        self.materialization = None;
    }

    pub fn assign_literal(&mut self, literal_fact: LiteralFact) {
        self.current_shape = Shape::Literal(literal_fact.clone());
        self.literal_fact = Some(literal_fact);
        self.materialization = None;
    }

    pub fn commit_materialization(&mut self, target: Shape) -> bool {
        let Some(literal_fact) = self.literal_fact.as_ref() else {
            return false;
        };
        if !can_materialize(literal_fact, &target) {
            return false;
        }

        self.current_shape = target.clone();
        self.materialization = Some(MaterializationPlan {
            target,
            commitment: Commitment::CommitBinding,
        });
        true
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct StateFrame {
    pub bindings: Vec<BindingState>,
}

impl StateFrame {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn define(&mut self, binding: BindingState) -> bool {
        if self.get(binding.key).is_some() {
            return false;
        }
        self.bindings.push(binding);
        true
    }

    #[must_use]
    pub fn get(&self, key: BindingKey) -> Option<&BindingState> {
        self.bindings.iter().find(|binding| binding.key == key)
    }

    pub fn get_mut(&mut self, key: BindingKey) -> Option<&mut BindingState> {
        self.bindings.iter_mut().find(|binding| binding.key == key)
    }

    pub fn assign_shape(&mut self, key: BindingKey, shape: Shape) -> bool {
        let Some(binding) = self.get_mut(key) else {
            return false;
        };
        binding.assign_shape(shape);
        true
    }

    pub fn assign_literal(&mut self, key: BindingKey, literal_fact: LiteralFact) -> bool {
        let Some(binding) = self.get_mut(key) else {
            return false;
        };
        binding.assign_literal(literal_fact);
        true
    }

    pub fn commit_materialization(&mut self, key: BindingKey, target: Shape) -> bool {
        self.get_mut(key)
            .is_some_and(|binding| binding.commit_materialization(target))
    }
}
