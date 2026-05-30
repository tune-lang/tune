use std::sync::{Arc, Mutex};

use tune_host::{HostResourceObject, ResourceCleanup, ResourceCleanupExecutor, ResourceRetention};
use tune_runtime::{ResourceHandle, ResourceId, ResourceTypeId};

#[derive(Debug, Clone)]
pub(crate) struct SharedResourceTable {
    inner: Arc<ResourceTableInner>,
}

impl Default for SharedResourceTable {
    fn default() -> Self {
        Self {
            inner: Arc::new(ResourceTableInner {
                resources: Mutex::new(Vec::new()),
            }),
        }
    }
}

impl SharedResourceTable {
    pub(crate) fn register(&self, handle: ResourceHandle, lifecycle: ResourceLifecycle) {
        self.register_with_object(handle, lifecycle, None);
    }

    pub(crate) fn register_with_object(
        &self,
        handle: ResourceHandle,
        lifecycle: ResourceLifecycle,
        object: Option<HostResourceObject>,
    ) {
        if !lifecycle.needs_tracking() && object.is_none() {
            return;
        }
        let mut resources = self
            .inner
            .resources
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let key = ResourceKey::from(&handle);
        if let Some(resource) = resources.iter_mut().find(|resource| resource.key == key) {
            if object.is_some() {
                resource.object = object;
            }
            return;
        }
        resources.push(ResourceRecord {
            key,
            handle,
            lifecycle,
            object,
        });
    }

    pub(crate) fn get_object(&self, handle: &ResourceHandle) -> Option<HostResourceObject> {
        let resources = self
            .inner
            .resources
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let key = ResourceKey::from(handle);
        resources
            .iter()
            .find(|resource| resource.key == key)
            .and_then(|resource| resource.object.clone())
    }

    pub(crate) fn cleanup_one(&self, handle: &ResourceHandle) -> Result<bool, String> {
        self.inner.cleanup_one(handle)
    }

    pub(crate) fn cleanup(&self) -> Result<(), String> {
        self.inner.cleanup()
    }
}

#[derive(Debug)]
struct ResourceTableInner {
    resources: Mutex<Vec<ResourceRecord>>,
}

impl ResourceTableInner {
    fn cleanup_one(&self, handle: &ResourceHandle) -> Result<bool, String> {
        let record = {
            let mut resources = self
                .resources
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            let key = ResourceKey::from(handle);
            let Some(index) = resources.iter().position(|resource| resource.key == key) else {
                return Ok(false);
            };
            resources.remove(index)
        };
        record.cleanup()?;
        Ok(true)
    }

    fn cleanup(&self) -> Result<(), String> {
        let records = {
            let mut resources = self
                .resources
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            resources.drain(..).collect::<Vec<_>>()
        };
        for record in records {
            record.cleanup()?;
        }
        Ok(())
    }
}

impl Drop for ResourceTableInner {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ResourceLifecycle {
    pub(crate) retention: ResourceRetention,
    pub(crate) cleanup: ResourceCleanup,
    pub(crate) cleanup_executor: Option<ResourceCleanupExecutor>,
}

impl ResourceLifecycle {
    pub(crate) fn needs_tracking(&self) -> bool {
        self.retention == ResourceRetention::Owned
            || matches!(self.cleanup, ResourceCleanup::HostCallback)
    }
}

#[derive(Debug, Clone)]
struct ResourceRecord {
    key: ResourceKey,
    handle: ResourceHandle,
    lifecycle: ResourceLifecycle,
    object: Option<HostResourceObject>,
}

impl ResourceRecord {
    fn cleanup(self) -> Result<(), String> {
        if self.lifecycle.cleanup != ResourceCleanup::HostCallback {
            return Ok(());
        }
        let Some(cleanup) = self.lifecycle.cleanup_executor else {
            return Ok(());
        };
        cleanup.cleanup(&self.handle).map_err(|error| error.message)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ResourceKey {
    id: ResourceId,
    type_id: Option<ResourceTypeId>,
}

impl From<&ResourceHandle> for ResourceKey {
    fn from(handle: &ResourceHandle) -> Self {
        Self {
            id: handle.id,
            type_id: handle.type_id,
        }
    }
}
