use tune_diagnostics::{Diagnostic, Span, codes};
use tune_hir::MemberId;
use tune_hir::item::{CallableMember, IndexAccess, Item, StructMember};

use super::super::{
    Analyzer, FiniteForContractKind,
    generics::{item_type_param_solution, substitute_generic_params},
};
use crate::{BindingKey, LiteralFact, Shape};

impl Analyzer<'_> {
    pub(in crate::analyze) fn iteration_contract(
        &mut self,
        shape: &Shape,
        span: Option<Span>,
    ) -> (FiniteForContractKind, Option<MemberId>, Option<MemberId>) {
        match shape {
            Shape::Hole => (FiniteForContractKind::Unknown, None, None),
            Shape::Sequence(_) | Shape::Literal(LiteralFact::Sequence { .. }) => {
                (FiniteForContractKind::Sequence, None, None)
            }
            Shape::Range(_) => (FiniteForContractKind::Range, None, None),
            Shape::Struct(nominal) | Shape::Apply { nominal, .. } => {
                let Some(id) = nominal.id else {
                    return (FiniteForContractKind::Unknown, None, None);
                };
                let len = self.callable_member(id, "len");
                let index = self.index_member(id);
                if len.is_none() || !self.len_member_returns_size(id) {
                    self.diagnostics.push(iter_diag(
                        codes::ITERATION_LEN_MISSING,
                        "finite `for` source has no `len(): Size` contract",
                        span,
                    ));
                }
                if index.is_none() || !self.index_member_accepts_size(id) {
                    self.diagnostics.push(iter_diag(
                        codes::ITERATION_INDEX_MISSING,
                        "finite `for` source has no indexed access contract",
                        span,
                    ));
                }
                (FiniteForContractKind::MemberAccess, len, index)
            }
            _ => {
                self.diagnostics.push(iter_diag(
                    codes::ITERATION_LEN_MISSING,
                    "finite `for` source has no `len(): Size` contract",
                    span,
                ));
                self.diagnostics.push(iter_diag(
                    codes::ITERATION_INDEX_MISSING,
                    "finite `for` source has no indexed access contract",
                    span,
                ));
                (FiniteForContractKind::Unknown, None, None)
            }
        }
    }

    pub(in crate::analyze) fn iteration_item_shape(
        &mut self,
        iterable: &Shape,
        index_member: Option<MemberId>,
    ) -> Shape {
        match iterable {
            Shape::Sequence(item) | Shape::Range(item) => item.as_ref().clone(),
            Shape::Literal(LiteralFact::Sequence { elements }) => {
                Shape::join_all(elements.iter().map(LiteralFact::storage_shape))
            }
            Shape::Struct(_) | Shape::Apply { .. } => index_member
                .and_then(|member| self.index_member_result_shape(member, iterable))
                .unwrap_or(Shape::Hole),
            _ => Shape::Hole,
        }
    }

    fn callable_member(&self, struct_id: tune_hir::HirId, member_name: &str) -> Option<MemberId> {
        self.struct_item(struct_id)?
            .struct_members
            .iter()
            .find_map(|member| match member {
                StructMember::Callable(callable)
                    if callable.name.as_deref() == Some(member_name) =>
                {
                    Some(callable.id)
                }
                _ => None,
            })
    }

    fn index_member(&self, struct_id: tune_hir::HirId) -> Option<MemberId> {
        self.struct_item(struct_id)?
            .struct_members
            .iter()
            .find_map(|member| match member {
                StructMember::IndexAccess(access) => Some(access.id),
                _ => None,
            })
    }

    fn len_member_returns_size(&mut self, struct_id: tune_hir::HirId) -> bool {
        let Some((owner, callable)) = self.callable_member_by_name(struct_id, "len") else {
            return false;
        };
        self.callable_member_return_shape(&owner, &callable) == Shape::Size
    }

    fn index_member_accepts_size(&mut self, struct_id: tune_hir::HirId) -> bool {
        let Some((owner, access)) = self.index_access_member(struct_id) else {
            return false;
        };
        self.index_access_param_shape(&owner, &access) == Shape::Size
    }

    fn callable_member_by_name(
        &self,
        struct_id: tune_hir::HirId,
        member_name: &str,
    ) -> Option<(Item, CallableMember)> {
        let item = self.struct_item(struct_id)?;
        item.struct_members.iter().find_map(|member| match member {
            StructMember::Callable(callable) if callable.name.as_deref() == Some(member_name) => {
                Some((item.clone(), callable.clone()))
            }
            _ => None,
        })
    }

    fn index_access_member(&self, struct_id: tune_hir::HirId) -> Option<(Item, IndexAccess)> {
        let item = self.struct_item(struct_id)?;
        item.struct_members.iter().find_map(|member| match member {
            StructMember::IndexAccess(access) => Some((item.clone(), access.clone())),
            _ => None,
        })
    }

    fn callable_member_return_shape(&mut self, owner: &Item, callable: &CallableMember) -> Shape {
        if let Some(shape) = &callable.shape {
            let lowered = super::super::item_shapes::lower_item_shape_expr(
                shape,
                owner,
                &self.resolved.scope,
            );
            self.diagnostics.extend(lowered.diagnostics);
            return lowered.shape;
        }
        let Some(body) = callable.body.as_ref() else {
            return Shape::Hole;
        };
        let mut analyzer = self.member_body_analyzer(owner);
        for param in &callable.params {
            analyzer.seed_member_param(param, owner);
        }
        let shape = analyzer.analyze_expr(body);
        self.diagnostics.extend(analyzer.diagnostics);
        shape
    }

    fn index_access_param_shape(&mut self, owner: &Item, access: &IndexAccess) -> Shape {
        if let Some(shape) = &access.index_shape {
            let lowered = super::super::item_shapes::lower_item_shape_expr(
                shape,
                owner,
                &self.resolved.scope,
            );
            self.diagnostics.extend(lowered.diagnostics);
            return lowered.shape;
        }
        let Some(body) = access.body.as_ref() else {
            return Shape::Hole;
        };
        let mut analyzer = self.member_body_analyzer(owner);
        analyzer.seed_member_param_shape(
            access.index_param_id,
            access.index_param_name.clone(),
            Shape::Hole,
            access.span,
        );
        analyzer.analyze_expr(body);
        let shape = analyzer
            .frame
            .get(BindingKey::Param(access.index_param_id))
            .map_or(Shape::Hole, |binding| {
                if binding.storage_shape == Shape::Hole {
                    binding.current_shape.clone()
                } else {
                    binding.storage_shape.clone()
                }
            });
        self.diagnostics.extend(analyzer.diagnostics);
        shape
    }

    fn index_member_result_shape(&mut self, member: MemberId, iterable: &Shape) -> Option<Shape> {
        let (owner, access) = self.index_access_member(member.owner)?;
        let mut shape = if let Some(result_shape) = &access.result_shape {
            let lowered = super::super::item_shapes::lower_item_shape_expr(
                result_shape,
                &owner,
                &self.resolved.scope,
            );
            self.diagnostics.extend(lowered.diagnostics);
            lowered.shape
        } else {
            let body = access.body.as_ref()?;
            let mut analyzer = self.member_body_analyzer(&owner);
            let index_shape = access
                .index_shape
                .as_ref()
                .map(|shape| {
                    super::super::item_shapes::lower_item_shape_expr(
                        shape,
                        &owner,
                        &self.resolved.scope,
                    )
                    .shape
                })
                .unwrap_or(Shape::Hole);
            analyzer.seed_member_param_shape(
                access.index_param_id,
                access.index_param_name.clone(),
                index_shape,
                access.span,
            );
            let shape = analyzer.analyze_expr(body);
            self.diagnostics.extend(analyzer.diagnostics);
            shape
        };

        if let Shape::Apply { args, .. } = iterable {
            let solved = item_type_param_solution(&owner, args);
            shape = substitute_generic_params(&shape, &solved);
        }
        Some(shape)
    }

    fn member_body_analyzer<'a>(&'a self, owner: &Item) -> Analyzer<'a> {
        let self_shape = owner_self_shape(owner);
        let mut analyzer = Analyzer {
            module: self.module,
            resolved: self.resolved,
            top_level_shapes: self.top_level_shapes,
            item_current_shape: self_shape.clone(),
            frame: crate::StateFrame::new(),
            expr_shapes: Vec::new(),
            calls: Vec::new(),
            returns: Vec::new(),
            assignments: Vec::new(),
            finite_for: Vec::new(),
            spawn: Vec::new(),
            materializers: Vec::new(),
            materializations: Vec::new(),
            diagnostics: Vec::new(),
            inferred_signature: None,
            expected_stack: Vec::new(),
        };
        analyzer.seed_self_value(self_shape, owner.span);
        analyzer
    }
}

fn iter_diag(
    code: tune_diagnostics::DiagnosticCode,
    title: &str,
    span: Option<Span>,
) -> Diagnostic {
    Diagnostic::error(
        code,
        title,
        span.unwrap_or_else(Span::synthetic),
        "finite `for` only lowers over sources with known length and indexed access",
    )
    .build()
}

fn owner_self_shape(owner: &Item) -> Shape {
    let Some(name) = owner.name.as_ref() else {
        return Shape::Hole;
    };
    let nominal = crate::NominalShape::new(owner.id, name);
    if owner.type_params.is_empty() {
        return Shape::Struct(nominal);
    }
    Shape::Apply {
        nominal,
        args: owner
            .type_params
            .iter()
            .map(|param| param.name.clone().map_or(Shape::Hole, Shape::Param))
            .collect(),
    }
}
