use tune_hir::expr::{BinaryOp, Expr};

use super::Analyzer;
use crate::Shape;

impl Analyzer<'_> {
    pub(super) fn analyze_binary(&mut self, op: BinaryOp, lhs: &Expr, rhs: &Expr) -> Shape {
        let lhs = self.analyze_expr(lhs);
        let rhs = self.analyze_expr(rhs);
        match op {
            BinaryOp::Or
            | BinaryOp::And
            | BinaryOp::BitOr
            | BinaryOp::BitXor
            | BinaryOp::BitAnd
                if Shape::Bool.accepts(&lhs) && Shape::Bool.accepts(&rhs) =>
            {
                Shape::Bool
            }
            BinaryOp::Or
            | BinaryOp::And
            | BinaryOp::BitOr
            | BinaryOp::BitXor
            | BinaryOp::BitAnd
                if Shape::Int.accepts(&lhs) && Shape::Int.accepts(&rhs) =>
            {
                Shape::Int
            }
            BinaryOp::Add
            | BinaryOp::Sub
            | BinaryOp::Mul
            | BinaryOp::Div
            | BinaryOp::Rem
            | BinaryOp::ShiftLeft
            | BinaryOp::ShiftRight
                if Shape::Int.accepts(&lhs) && Shape::Int.accepts(&rhs) =>
            {
                Shape::Int
            }
            BinaryOp::Is
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
            | BinaryOp::Or
            | BinaryOp::And
            | BinaryOp::BitOr
            | BinaryOp::BitXor
            | BinaryOp::BitAnd
            | BinaryOp::ShiftLeft
            | BinaryOp::ShiftRight => Shape::Hole,
        }
    }
}
