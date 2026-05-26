pub mod facts;
pub mod imports;
pub mod resolve;
pub mod scope;

pub use resolve::{ResolvedModule, resolve_module};
pub use scope::{Binding, BindingKind, Scope};
