//! Core crate for the dyno workspace.

/// Returns the workspace crate name.
#[must_use]
pub const fn name() -> &'static str {
    "dyno"
}
