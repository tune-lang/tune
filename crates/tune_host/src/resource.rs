use std::fmt;
use std::sync::Arc;

use tune_shape::Shape;

use crate::{Authority, HostCallError};
use tune_runtime::ResourceHandle;

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
    pub cleanup_executor: Option<ResourceCleanupExecutor>,
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
            cleanup_executor: None,
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
    pub fn with_cleanup_executor(mut self, cleanup: impl ResourceCleaner + 'static) -> Self {
        self.cleanup_executor = Some(ResourceCleanupExecutor::new(cleanup));
        self
    }

    #[must_use]
    pub fn task_safe(mut self, task_safe: bool) -> Self {
        self.task_safe = task_safe;
        self
    }
}

pub type ResourceCleanupResult = Result<(), HostCallError>;

pub trait ResourceCleaner: Send + Sync {
    fn cleanup(&self, resource: &ResourceHandle) -> ResourceCleanupResult;
}

impl<F> ResourceCleaner for F
where
    F: Fn(&ResourceHandle) -> ResourceCleanupResult + Send + Sync,
{
    fn cleanup(&self, resource: &ResourceHandle) -> ResourceCleanupResult {
        self(resource)
    }
}

#[derive(Clone)]
pub struct ResourceCleanupExecutor {
    cleaner: Arc<dyn ResourceCleaner>,
}

impl ResourceCleanupExecutor {
    #[must_use]
    pub fn new(cleaner: impl ResourceCleaner + 'static) -> Self {
        Self {
            cleaner: Arc::new(cleaner),
        }
    }

    pub fn cleanup(&self, resource: &ResourceHandle) -> ResourceCleanupResult {
        self.cleaner.cleanup(resource)
    }
}

impl fmt::Debug for ResourceCleanupExecutor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ResourceCleanupExecutor")
            .finish_non_exhaustive()
    }
}

impl PartialEq for ResourceCleanupExecutor {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.cleaner, &other.cleaner)
    }
}
