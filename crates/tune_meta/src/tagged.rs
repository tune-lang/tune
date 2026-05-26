#[derive(Debug, Clone)]
pub struct TaggedDecl<TTag> {
    pub tag: TTag,
    pub decl_id: u32,
}
