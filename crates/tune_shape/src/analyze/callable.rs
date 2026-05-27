use tune_hir::expr::{Expr, ExprParam};

use super::Analyzer;
use crate::{BindingKey, BindingState, Shape, lower_resolved_hir_shape};

impl Analyzer<'_> {
    pub(super) fn analyze_callable_value(&mut self, params: &[ExprParam], body: &Expr) -> Shape {
        let outer_frame = self.frame.clone();
        let return_start = self.returns.len();
        self.frame = outer_frame.clone();

        let param_shapes = params
            .iter()
            .map(|param| self.bind_callable_param(param))
            .collect::<Vec<_>>();

        let body_shape = self.analyze_expr(body);
        let return_shapes = self.returns[return_start..]
            .iter()
            .map(|ret| ret.shape.clone())
            .collect::<Vec<_>>();
        self.returns.truncate(return_start);
        self.frame = outer_frame;

        Shape::Callable {
            params: param_shapes,
            ret: Box::new(Shape::join_all(
                [body_shape].into_iter().chain(return_shapes),
            )),
        }
    }

    fn bind_callable_param(&mut self, param: &ExprParam) -> Shape {
        let shape = param
            .shape
            .as_ref()
            .map(|shape| lower_resolved_hir_shape(shape, &self.resolved.scope))
            .map_or(Shape::Hole, |lowered| {
                self.diagnostics.extend(lowered.diagnostics);
                lowered.shape
            });
        if let Some(name) = &param.name
            && let Some(local) = self.callable_param_local(name, param.span)
        {
            self.frame.define(BindingState::new(
                BindingKey::Local(local),
                Some(name.clone()),
                shape.clone(),
                shape.clone(),
                param.span,
            ));
        }
        shape
    }
}
