pub mod constraints;
pub mod hir;
pub mod literal;
pub mod materialize;
pub mod shape;
pub mod state;

pub use hir::{intern_hir_shape, lower_hir_shape};
pub use literal::LiteralFact;
pub use materialize::{Commitment, MaterializationPlan, can_materialize};
pub use shape::{Shape, ShapeFact, ShapeId, ShapeOrigin, ShapeStore};
