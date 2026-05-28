use tune_hir::pattern::{Pattern, PatternKind};
use tune_resolve::VariantId;

use super::LowerContext;
use crate::plan::PlanPatternBinding;

impl LowerContext<'_> {
    pub(super) fn for_pattern_binding(&self, pattern: &Pattern) -> Option<tune_resolve::LocalId> {
        let PatternKind::Binding(_) = &pattern.kind else {
            return None;
        };
        self.local_for_expr(pattern.id)
    }

    pub(super) fn pattern_variant(&self, pattern: &Pattern) -> Option<VariantId> {
        self.resolved?
            .variant_pattern_refs
            .iter()
            .find(|variant_ref| variant_ref.pattern == pattern.id)
            .map(|variant_ref| variant_ref.variant)
    }

    pub(super) fn pattern_bindings(&self, pattern: &Pattern) -> Vec<PlanPatternBinding> {
        let PatternKind::Variant { args, .. } = &pattern.kind else {
            return Vec::new();
        };

        args.iter()
            .enumerate()
            .filter_map(|(field_index, arg)| {
                let PatternKind::Binding(_) = arg.kind else {
                    return None;
                };
                Some(PlanPatternBinding {
                    local: self.local_for_expr(arg.id),
                    field_index,
                })
            })
            .collect()
    }
}
