use tune_runtime::{ResourceHandle, ResourceTypeId, Value};

use crate::{Vm, VmError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VmHostResourceType {
    pub id: ResourceTypeId,
    pub type_name: String,
    pub task_safe: bool,
    pub authorities: Vec<tune_host::Authority>,
}

impl VmHostResourceType {
    #[must_use]
    pub fn new(id: ResourceTypeId, type_name: impl Into<String>) -> Self {
        Self {
            id,
            type_name: type_name.into(),
            task_safe: false,
            authorities: Vec::new(),
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

        Ok(ResourceHandle {
            id: resource.id,
            type_id: Some(resource_type.id),
            type_name: resource_type.type_name.clone(),
            task_safe: resource_type.task_safe,
        })
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
