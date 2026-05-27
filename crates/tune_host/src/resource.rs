use tune_shape::Shape;

use crate::Authority;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResourceRetention {
    Borrowed,
    Owned,
    HostRetained,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResourceCleanup {
    None,
    Close,
    HostCallback,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HostResourceType {
    pub name: String,
    pub shape: Shape,
    pub authorities: Vec<Authority>,
    pub retention: ResourceRetention,
    pub cleanup: ResourceCleanup,
    pub task_safe: bool,
}

impl HostResourceType {
    #[must_use]
    pub fn new(name: impl Into<String>, shape: Shape) -> Self {
        Self {
            name: name.into(),
            shape,
            authorities: Vec::new(),
            retention: ResourceRetention::Owned,
            cleanup: ResourceCleanup::Close,
            task_safe: false,
        }
    }

    #[must_use]
    pub fn with_authorities(mut self, authorities: Vec<Authority>) -> Self {
        self.authorities = authorities;
        self
    }

    #[must_use]
    pub fn retention(mut self, retention: ResourceRetention) -> Self {
        self.retention = retention;
        self
    }

    #[must_use]
    pub fn cleanup(mut self, cleanup: ResourceCleanup) -> Self {
        self.cleanup = cleanup;
        self
    }

    #[must_use]
    pub fn task_safe(mut self, task_safe: bool) -> Self {
        self.task_safe = task_safe;
        self
    }
}
