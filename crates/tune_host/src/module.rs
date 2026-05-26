use crate::function::HostFunction;

#[derive(Debug, Clone)]
pub struct HostModule {
    pub name: String,
    pub functions: Vec<HostFunction>,
}
