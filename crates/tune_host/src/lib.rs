pub mod authority;
pub mod function;
pub mod module;
pub mod resource;
pub mod symbol;
pub mod value_type;

pub use authority::Authority;
pub use function::{
    HostCallError, HostCallResult, HostContext, HostExecutor, HostFunction, HostParam,
    HostResourceObject, downcast_resource,
};
pub use module::HostModule;
pub use resource::{
    HostResourceType, ResourceCleaner, ResourceCleanup, ResourceCleanupExecutor,
    ResourceCleanupResult, ResourceRetention,
};
pub use symbol::HostSymbolId;
pub use value_type::{HostValueField, HostValueType};

pub trait Host {
    fn modules(&self) -> Vec<module::HostModule> {
        Vec::new()
    }
}
