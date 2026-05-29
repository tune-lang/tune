use crate::function::HostFunction;

#[derive(Debug, Clone)]
pub struct HostModule {
    pub name: String,
    pub functions: Vec<HostFunction>,
}

impl HostModule {
    #[must_use]
    pub fn new(name: impl Into<String>, functions: Vec<HostFunction>) -> Self {
        Self {
            name: name.into(),
            functions,
        }
    }
}
