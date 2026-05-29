use tune_hir::HirId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonInvokerPlan {
    pub decl_id: HirId,
    pub helper_name: String,
    pub uses_runtime_reflection: bool,
}

#[must_use]
pub fn generate_json_invoker(decl_id: HirId) -> JsonInvokerPlan {
    JsonInvokerPlan {
        decl_id,
        helper_name: format!("__json_invoker_{}", decl_id.0),
        uses_runtime_reflection: false,
    }
}
