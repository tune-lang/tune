use tune_hir::HirId;

#[derive(Debug, Clone)]
pub struct TaggedDecl<TTag> {
    pub tag: TTag,
    pub decl_id: HirId,
}
