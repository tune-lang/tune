use crate::shape::MemberRequirement;

#[derive(Debug, Clone)]
pub struct StructuralConstraint {
    pub requirements: Vec<MemberRequirement>,
}
