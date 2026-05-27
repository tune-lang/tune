use tune_hir::expr::{BinaryOp, Expr};

use super::Analyzer;
use crate::Shape;

impl Analyzer<'_> {
    pub(super) fn analyze_binary(&mut self, op: BinaryOp, lhs: &Expr, rhs: &Expr) -> Shape {
        let lhs = self.analyze_expr(lhs);
        let rhs = self.analyze_expr(rhs);
        match op {
            BinaryOp::Add
            | BinaryOp::Sub
            | BinaryOp::Mul
            | BinaryOp::Div
            | BinaryOp::Rem
            | BinaryOp::BitOr
            | BinaryOp::BitXor
            | BinaryOp::BitAnd
            | BinaryOp::ShiftLeft
            | BinaryOp::ShiftRight
                if Shape::Int.accepts(&lhs) && Shape::Int.accepts(&rhs) =>
            {
                Shape::Int
            }
            BinaryOp::Or
            | BinaryOp::And
            | BinaryOp::Is
            | BinaryOp::IsNot
            | BinaryOp::Equal
            | BinaryOp::NotEqual
            | BinaryOp::Less
            | BinaryOp::LessEqual
            | BinaryOp::Greater
            | BinaryOp::GreaterEqual => Shape::Bool,
            BinaryOp::Add
            | BinaryOp::Sub
            | BinaryOp::Mul
            | BinaryOp::Div
            | BinaryOp::Rem
            | BinaryOp::BitOr
            | BinaryOp::BitXor
            | BinaryOp::BitAnd
            | BinaryOp::ShiftLeft
            | BinaryOp::ShiftRight => Shape::Hole,
        }
    }
}
