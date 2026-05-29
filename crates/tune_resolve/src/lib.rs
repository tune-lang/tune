pub mod facts;
pub mod imports;
pub mod locals;
pub mod prelude;
pub mod resolve;
pub mod scope;

pub use facts::{
    CompilerFact, CompilerFactKind, CompilerFactPayload, FactOwner, TagFact, TagFactArg,
};
pub use imports::ImportKind;
pub use locals::{LocalBinding, LocalId, LocalKind, NameRef, NameTarget, VariantPatternRef};
pub use prelude::{Prelude, PreludeType, PreludeVariant, VariantId};
pub use resolve::{ResolvedModule, resolve_module};
pub use scope::{Binding, BindingKind, Scope};
