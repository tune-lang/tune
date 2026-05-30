use tune_runtime::{ResourceHandle, ResourceId, ResourceTypeId, Value};

use crate::{Vm, VmError, resource_table::ResourceLifecycle};

#[derive(Debug, Clone, PartialEq)]
pub struct VmHostResourceType {
    pub id: ResourceTypeId,
    pub type_name: String,
    pub task_safe: bool,
    pub authorities: Vec<tune_host::Authority>,
    pub retention: tune_host::ResourceRetention,
    pub cleanup: tune_host::ResourceCleanup,
    pub cleanup_executor: Option<tune_host::ResourceCleanupExecutor>,
}

impl VmHostResourceType {
    #[must_use]
    pub fn new(id: ResourceTypeId, type_name: impl Into<String>) -> Self {
        Self {
            id,
            type_name: type_name.into(),
            task_safe: false,
            authorities: Vec::new(),
            retention: tune_host::ResourceRetention::Owned,
            cleanup: tune_host::ResourceCleanup::Close,
            cleanup_executor: None,
        }
    }

    #[must_use]
    pub fn task_safe(mut self, task_safe: bool) -> Self {
        self.task_safe = task_safe;
        self
    }

    #[must_use]
    pub fn with_authorities(mut self, authorities: Vec<tune_host::Authority>) -> Self {
        self.authorities = authorities;
        self
    }

    #[must_use]
    pub fn retention(mut self, retention: tune_host::ResourceRetention) -> Self {
        self.retention = retention;
        self
    }

    #[must_use]
    pub fn cleanup(mut self, cleanup: tune_host::ResourceCleanup) -> Self {
        self.cleanup = cleanup;
        self
    }

    #[must_use]
    pub fn with_cleanup_executor(mut self, cleanup: tune_host::ResourceCleanupExecutor) -> Self {
        self.cleanup_executor = Some(cleanup);
        self
    }

    #[must_use]
    pub fn with_cleanup_executor_if_present(
        mut self,
        cleanup: Option<tune_host::ResourceCleanupExecutor>,
    ) -> Self {
        self.cleanup_executor = cleanup;
        self
    }
}

impl Vm {
    pub(crate) fn normalize_host_value(&self, value: Value) -> Result<Value, VmError> {
        match value {
            Value::Resource(resource) => {
                self.normalize_host_resource(resource).map(Value::Resource)
            }
            Value::Sequence(values) => values
                .into_iter()
                .map(|value| self.normalize_host_value(value))
                .collect::<Result<Vec<_>, _>>()
                .map(Value::Sequence),
            Value::Tuple(values) => values
                .into_iter()
                .map(|value| self.normalize_host_value(value))
                .collect::<Result<Vec<_>, _>>()
                .map(Value::Tuple),
            Value::Variant {
                variant,
                fields,
                propagation_frames,
            } => fields
                .into_iter()
                .map(|field| self.normalize_host_value(field))
                .collect::<Result<Vec<_>, _>>()
                .map(|fields| Value::Variant {
                    variant,
                    fields,
                    propagation_frames,
                }),
            value => Ok(value),
        }
    }

    fn normalize_host_resource(&self, resource: ResourceHandle) -> Result<ResourceHandle, VmError> {
        let Some(resource_type) = self.resolve_host_resource_type(&resource) else {
            if self.host_resource_types.is_empty() {
                return Ok(resource);
            }
            return Err(VmError::UnknownHostResourceType {
                resource_type: resource.type_name,
            });
        };

        for authority in &resource_type.authorities {
            if !self.granted_authorities.contains(authority) {
                return Err(VmError::MissingHostAuthority {
                    authority: authority.0.clone(),
                });
            }
        }

        let normalized = ResourceHandle {
            id: resource.id,
            type_id: Some(resource_type.id),
            type_name: resource_type.type_name.clone(),
            task_safe: resource_type.task_safe,
        };
        self.resources.register(
            normalized.clone(),
            ResourceLifecycle {
                retention: resource_type.retention.clone(),
                cleanup: resource_type.cleanup.clone(),
                cleanup_executor: resource_type.cleanup_executor.clone(),
            },
        );
        Ok(normalized)
    }

    fn resolve_host_resource_type(&self, resource: &ResourceHandle) -> Option<&VmHostResourceType> {
        if let Some(type_id) = resource.type_id {
            return self
                .host_resource_types
                .iter()
                .find(|resource_type| resource_type.id == type_id);
        }
        self.host_resource_types
            .iter()
            .find(|resource_type| resource_type.type_name == resource.type_name)
    }
}

impl tune_host::HostContext for Vm {
    fn insert_resource(
        &self,
        type_name: &str,
        object: tune_host::HostResourceObject,
    ) -> Result<ResourceHandle, tune_host::HostCallError> {
        let id = ResourceId(
            self.next_resource_id
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed),
        );
        let handle = ResourceHandle::new(id, type_name);
        let normalized = self
            .normalize_host_resource(handle)
            .map_err(|error| tune_host::HostCallError::new(format!("{error:?}")))?;
        let resource_type = self
            .resolve_host_resource_type(&normalized)
            .ok_or_else(|| tune_host::HostCallError::new("unknown host resource type"))?;
        self.resources.register_with_object(
            normalized.clone(),
            ResourceLifecycle {
                retention: resource_type.retention.clone(),
                cleanup: resource_type.cleanup.clone(),
                cleanup_executor: resource_type.cleanup_executor.clone(),
            },
            Some(object),
        );
        Ok(normalized)
    }

    fn get_resource(
        &self,
        handle: &ResourceHandle,
    ) -> Result<tune_host::HostResourceObject, tune_host::HostCallError> {
        self.resources
            .get_object(handle)
            .ok_or_else(|| tune_host::HostCallError::new("host resource is closed or unknown"))
    }

    fn close_resource(&self, handle: &ResourceHandle) -> Result<(), tune_host::HostCallError> {
        self.resources
            .cleanup_one(handle)
            .map_err(tune_host::HostCallError::new)
            .map(|_| ())
    }
}
