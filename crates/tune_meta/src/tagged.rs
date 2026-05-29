use tune_hir::HirId;

#[derive(Debug, Clone)]
pub struct TaggedDecl<TTag> {
    pub tag: TTag,
    pub decl_id: HirId,
}

#[must_use]
pub fn tagged_decls(
    tag_name: &str,
    facts: &[tune_resolve::CompilerFact],
) -> Vec<TaggedDecl<tune_resolve::TagFact>> {
    facts
        .iter()
        .filter_map(|fact| {
            let tune_resolve::FactOwner::Item(decl_id) = fact.owner else {
                return None;
            };
            let tune_resolve::CompilerFactPayload::Tag(tag) = &fact.payload else {
                return None;
            };
            (tag.name == tag_name).then(|| TaggedDecl {
                tag: tag.clone(),
                decl_id,
            })
        })
        .collect()
}
