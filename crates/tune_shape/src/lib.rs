pub mod analyze;
pub mod constraints;
pub mod expr;
pub mod flow;
pub mod hir;
pub mod literal;
pub mod materialize;
pub mod shape;
pub mod state;

pub use analyze::{
    AssignmentCheck, CallCheck, CallSignature, CallTarget, ExprShape, FiniteForCheck,
    FiniteForContractKind, MaterializerCheck, ReturnCheck, ShapeAnalysis, analyze_item,
    analyze_module,
};
pub use expr::{expr_literal_fact, expr_shape_fact};
pub use flow::{expr_propagated_error_shape_fact, expr_result_constructor_shape_fact};
pub use hir::{
    LoweredShape, alloc_hir_shape, alloc_resolved_hir_shape, lower_hir_shape,
    lower_resolved_hir_shape,
};
pub use literal::LiteralFact;
pub use materialize::{Commitment, MaterializationPlan, can_materialize};
pub use shape::{MemberRequirement, Shape, ShapeFact, ShapeId, ShapeOrigin, ShapeStore};
pub use state::{BindingKey, BindingState, StateFrame, StateJoinError};
