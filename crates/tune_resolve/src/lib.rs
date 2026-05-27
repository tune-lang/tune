pub mod facts;
pub mod imports;
pub mod locals;
pub mod resolve;
pub mod scope;

pub use facts::{
    CompilerFact, CompilerFactKind, CompilerFactPayload, FactOwner, TagFact, TagFactArg,
};
pub use locals::{LocalBinding, LocalId, LocalKind, NameRef, NameTarget};
pub use resolve::{ResolvedModule, resolve_module};
pub use scope::{Binding, BindingKind, Scope};
