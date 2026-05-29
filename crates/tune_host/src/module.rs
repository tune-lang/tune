use crate::function::HostFunction;
use crate::resource::HostResourceType;

#[derive(Debug, Clone)]
pub struct HostModule {
    pub name: String,
    pub functions: Vec<HostFunction>,
    pub resources: Vec<HostResourceType>,
}

impl HostModule {
    #[must_use]
    pub fn new(name: impl Into<String>, functions: Vec<HostFunction>) -> Self {
        Self {
            name: name.into(),
            functions,
            resources: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_resources(mut self, resources: Vec<HostResourceType>) -> Self {
        self.resources = resources;
        self
    }
}
