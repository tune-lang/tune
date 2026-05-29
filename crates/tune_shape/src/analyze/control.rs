use tune_hir::expr::{BinaryOp, Expr, ExprKind, LiteralKind};
use tune_hir::pattern::{Pattern, PatternKind};

use super::{Analyzer, FiniteForCheck};
use crate::{BindingKey, LiteralFact, Shape, StateFrame};

impl Analyzer<'_> {
    pub(super) fn analyze_match(
        &mut self,
        expr: &Expr,
        scrutinee: &Expr,
        arms: &[tune_hir::expr::MatchArm],
    ) -> Shape {
        let scrutinee_shape = self.analyze_expr(scrutinee);
        self.check_match_exhaustive(expr, &scrutinee_shape, arms);
        let entry = self.frame.clone();
        let mut frames = Vec::new();
        let mut shapes = Vec::new();
        for arm in arms {
            self.frame = entry.clone();
            self.apply_structural_pattern(scrutinee, &arm.pattern, &scrutinee_shape);
            self.apply_none_pattern(scrutinee, &arm.pattern);
            self.bind_pattern(&arm.pattern, Shape::Hole);
            let shape = self.analyze_expr(&arm.body);
            if shape != Shape::Never {
                frames.push(self.frame.clone());
            }
            shapes.push(shape);
        }
        self.join_branch_frames(entry, frames);
        join_continuing_shapes(shapes)
    }

    pub(super) fn analyze_match_expected(
        &mut self,
        expr: &Expr,
        scrutinee: &Expr,
        arms: &[tune_hir::expr::MatchArm],
        expected: &Shape,
    ) -> Shape {
        let scrutinee_shape = self.analyze_expr(scrutinee);
        self.check_match_exhaustive(expr, &scrutinee_shape, arms);
        let entry = self.frame.clone();
        let mut frames = Vec::new();
        let mut shapes = Vec::new();
        for arm in arms {
            self.frame = entry.clone();
            self.apply_structural_pattern(scrutinee, &arm.pattern, &scrutinee_shape);
            self.apply_none_pattern(scrutinee, &arm.pattern);
            self.bind_pattern(&arm.pattern, Shape::Hole);
            let shape = self.analyze_expr_expected(&arm.body, expected);
            if shape != Shape::Never {
                frames.push(self.frame.clone());
            }
            shapes.push(shape);
        }
        self.join_branch_frames(entry, frames);
        join_continuing_shapes(shapes)
    }

    pub(super) fn analyze_for(
        &mut self,
        expr: &Expr,
        pattern: &Pattern,
        iterable: &Expr,
        body: &Expr,
    ) -> Shape {
        let iterable_shape = self.analyze_expr(iterable);
        let (contract, len_member, index_member) =
            self.iteration_contract(&iterable_shape, expr.span);
        self.finite_for.push(FiniteForCheck {
            iterable: iterable.id,
            contract,
            len_member,
            index_member,
            span: expr.span,
        });
        self.check_iteration_source_mutation(iterable, body, expr.span);
        let entry = self.frame.clone();
        self.frame = entry.clone();
        self.bind_pattern(pattern, iteration_item_shape(&iterable_shape));
        self.analyze_expr(body);
        let body_frame = self.frame.clone();
        self.frame = entry;
        let _ = self.frame.join_from(&body_frame);
        Shape::Unit
    }

    pub(super) fn analyze_if(
        &mut self,
        branches: &[tune_hir::expr::IfBranch],
        else_branch: Option<&Expr>,
    ) -> Shape {
        let entry = self.frame.clone();
        let mut frames = Vec::new();
        let mut shapes = Vec::new();
        for branch in branches {
            self.frame = entry.clone();
            self.analyze_expr(&branch.condition);
            self.apply_condition_narrowing(&branch.condition, true);
            let shape = self.analyze_expr(&branch.body);
            if shape != Shape::Never {
                frames.push(self.frame.clone());
            }
            shapes.push(shape);
        }
        if let Some(else_branch) = else_branch {
            self.frame = entry.clone();
            for branch in branches {
                self.apply_condition_narrowing(&branch.condition, false);
            }
            let shape = self.analyze_expr(else_branch);
            if shape != Shape::Never {
                frames.push(self.frame.clone());
            }
            shapes.push(shape);
        } else {
            self.frame = entry.clone();
            for branch in branches {
                self.apply_condition_narrowing(&branch.condition, false);
            }
            shapes.push(Shape::Hole);
            frames.push(self.frame.clone());
        }
        self.join_branch_frames(entry, frames);
        join_continuing_shapes(shapes)
    }

    pub(super) fn analyze_if_expected(
        &mut self,
        branches: &[tune_hir::expr::IfBranch],
        else_branch: Option<&Expr>,
        expected: &Shape,
    ) -> Shape {
        let entry = self.frame.clone();
        let mut frames = Vec::new();
        let mut shapes = Vec::new();
        for branch in branches {
            self.frame = entry.clone();
            self.analyze_expr(&branch.condition);
            self.apply_condition_narrowing(&branch.condition, true);
            let shape = self.analyze_expr_expected(&branch.body, expected);
            if shape != Shape::Never {
                frames.push(self.frame.clone());
            }
            shapes.push(shape);
        }
        if let Some(else_branch) = else_branch {
            self.frame = entry.clone();
            for branch in branches {
                self.apply_condition_narrowing(&branch.condition, false);
            }
            let shape = self.analyze_expr_expected(else_branch, expected);
            if shape != Shape::Never {
                frames.push(self.frame.clone());
            }
            shapes.push(shape);
        } else {
            self.frame = entry.clone();
            for branch in branches {
                self.apply_condition_narrowing(&branch.condition, false);
            }
            shapes.push(Shape::Hole);
            frames.push(self.frame.clone());
        }
        self.join_branch_frames(entry, frames);
        join_continuing_shapes(shapes)
    }

    pub(super) fn analyze_while(&mut self, condition: &Expr, body: &Expr) -> Shape {
        self.analyze_expr(condition);
        let entry = self.frame.clone();
        self.frame = entry.clone();
        self.analyze_expr(body);
        let body_frame = self.frame.clone();
        self.frame = entry;
        let _ = self.frame.join_from(&body_frame);
        Shape::Unit
    }

    pub(super) fn analyze_loop(&mut self, body: &Expr) -> Shape {
        let entry = self.frame.clone();
        self.frame = entry.clone();
        let body_shape = self.analyze_expr(body);
        self.frame = entry;
        if body_shape == Shape::Never {
            Shape::Never
        } else {
            Shape::Hole
        }
    }

    fn join_branch_frames(&mut self, entry: StateFrame, mut frames: Vec<StateFrame>) {
        let Some(mut joined) = frames.pop() else {
            self.frame = entry;
            return;
        };
        for frame in &frames {
            let _ = joined.join_from(frame);
        }
        self.frame = joined;
    }

    pub(super) fn apply_condition_narrowing(&mut self, condition: &Expr, truthy: bool) {
        for (key, narrowed) in optional_none_narrowings(condition, truthy, self) {
            if let Some(binding) = self.frame.get_mut(key) {
                binding.narrow_current(narrowed);
            }
        }
    }

    fn apply_none_pattern(&mut self, scrutinee: &Expr, pattern: &Pattern) {
        if !matches!(pattern.kind, PatternKind::None) {
            return;
        }
        let Some(key) = self.binding_key(scrutinee) else {
            return;
        };
        if let Some(binding) = self.frame.get_mut(key) {
            binding.narrow_current(Shape::Literal(LiteralFact::None));
        }
    }
}

fn optional_none_narrowings(
    condition: &Expr,
    truthy: bool,
    analyzer: &Analyzer<'_>,
) -> Vec<(BindingKey, Shape)> {
    let ExprKind::Binary { op, lhs, rhs } = &condition.kind else {
        return Vec::new();
    };
    match (op, truthy) {
        (BinaryOp::And, true) | (BinaryOp::Or, false) => {
            let mut narrowings = optional_none_narrowings(lhs, truthy, analyzer);
            narrowings.extend(optional_none_narrowings(rhs, truthy, analyzer));
            return narrowings;
        }
        _ => {}
    }
    let is_equal = matches!(op, BinaryOp::Equal);
    let is_not_equal = matches!(op, BinaryOp::NotEqual);
    if !is_equal && !is_not_equal {
        return Vec::new();
    }

    let value = if is_none_literal(rhs) {
        lhs
    } else if is_none_literal(lhs) {
        rhs
    } else {
        return Vec::new();
    };
    let Some(key) = analyzer.binding_key(value) else {
        return Vec::new();
    };
    let Some(binding) = analyzer.frame.get(key) else {
        return Vec::new();
    };
    let Shape::Optional(payload) = &binding.current_shape else {
        return Vec::new();
    };

    let narrows_to_payload = is_not_equal == truthy;
    let narrowed = if narrows_to_payload {
        payload.as_ref().clone()
    } else {
        Shape::Literal(LiteralFact::None)
    };
    vec![(key, narrowed)]
}

fn is_none_literal(expr: &Expr) -> bool {
    matches!(expr.kind, ExprKind::Literal(LiteralKind::None))
}

fn join_continuing_shapes(shapes: Vec<Shape>) -> Shape {
    let mut saw_never = false;
    let continuing = shapes
        .into_iter()
        .filter(|shape| {
            let is_never = *shape == Shape::Never;
            saw_never |= is_never;
            !is_never
        })
        .collect::<Vec<_>>();

    if continuing.is_empty() && saw_never {
        Shape::Never
    } else {
        Shape::join_all(continuing)
    }
}

fn iteration_item_shape(iterable: &Shape) -> Shape {
    match iterable {
        Shape::Sequence(item) | Shape::Range(item) => item.as_ref().clone(),
        _ => Shape::Hole,
    }
}
