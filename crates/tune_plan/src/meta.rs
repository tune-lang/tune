use tune_hir::HirId;
use tune_resolve::{CompilerFactKind, FactOwner};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MetaPlan {
    StaticTaggedTable {
        tag: HirId,
    },
    CompilerFact {
        owner: FactOwner,
        kind: CompilerFactKind,
    },
    GeneratedJsonInvoker {
        function: HirId,
    },
}
