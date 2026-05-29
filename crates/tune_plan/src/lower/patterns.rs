use tune_hir::pattern::{Pattern, PatternKind};
use tune_resolve::VariantId;

use super::LowerContext;
use crate::plan::{PlanPatternBinding, PlanPatternPathSegment, PlanPatternTest};

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
        let mut bindings = Vec::new();
        collect_pattern_bindings(self, pattern, &mut Vec::new(), &mut bindings);
        bindings
    }

    pub(super) fn pattern_tests(&self, pattern: &Pattern) -> Vec<PlanPatternTest> {
        let mut tests = Vec::new();
        collect_pattern_tests(self, pattern, &mut Vec::new(), &mut tests);
        tests
    }
}

fn collect_pattern_bindings(
    context: &LowerContext<'_>,
    pattern: &Pattern,
    path: &mut Vec<PlanPatternPathSegment>,
    bindings: &mut Vec<PlanPatternBinding>,
) {
    match &pattern.kind {
        PatternKind::Binding(_) => bindings.push(PlanPatternBinding {
            local: context.local_for_expr(pattern.id),
            field_path: path.clone(),
        }),
        PatternKind::Variant { args, .. } => {
            for (index, arg) in args.iter().enumerate() {
                path.push(PlanPatternPathSegment::VariantField(index));
                collect_pattern_bindings(context, arg, path, bindings);
                path.pop();
            }
        }
        PatternKind::Tuple(args) => {
            for (index, arg) in args.iter().enumerate() {
                path.push(PlanPatternPathSegment::TupleField(index));
                collect_pattern_bindings(context, arg, path, bindings);
                path.pop();
            }
        }
        PatternKind::Hole
        | PatternKind::Unit
        | PatternKind::StructuralShape(_)
        | PatternKind::Else => {}
    }
}

fn collect_pattern_tests(
    context: &LowerContext<'_>,
    pattern: &Pattern,
    path: &mut Vec<PlanPatternPathSegment>,
    tests: &mut Vec<PlanPatternTest>,
) {
    match &pattern.kind {
        PatternKind::Variant { args, .. } => {
            if let Some(variant) = context.pattern_variant(pattern) {
                tests.push(PlanPatternTest {
                    field_path: path.clone(),
                    variant,
                });
            }
            for (index, arg) in args.iter().enumerate() {
                path.push(PlanPatternPathSegment::VariantField(index));
                collect_pattern_tests(context, arg, path, tests);
                path.pop();
            }
        }
        PatternKind::Tuple(args) => {
            for (index, arg) in args.iter().enumerate() {
                path.push(PlanPatternPathSegment::TupleField(index));
                collect_pattern_tests(context, arg, path, tests);
                path.pop();
            }
        }
        PatternKind::Hole
        | PatternKind::Binding(_)
        | PatternKind::Unit
        | PatternKind::StructuralShape(_)
        | PatternKind::Else => {}
    }
}
