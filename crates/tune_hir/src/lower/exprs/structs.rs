use tune_ast::nodes::StructExpr;

use super::ExprLowerer;
use crate::expr::{ExprKind, StructFieldInit};

impl ExprLowerer {
    pub(super) fn lower_struct(&mut self, source: &str, node: StructExpr<'_>) -> ExprKind {
        ExprKind::Struct {
            name: node.name(source).unwrap_or_default().to_owned(),
            fields: node
                .fields()
                .into_iter()
                .filter_map(|field| {
                    Some(StructFieldInit {
                        name: field.name(source)?.to_owned(),
                        value: self.lower(source, field.value()?),
                    })
                })
                .collect(),
        }
    }
}
