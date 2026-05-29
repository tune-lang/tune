pub mod authority;
pub mod function;
pub mod module;
pub mod resource;

pub use authority::Authority;
pub use function::{HostCallError, HostCallResult, HostExecutor, HostFunction, HostParam};
pub use module::HostModule;
pub use resource::{HostResourceType, ResourceCleanup, ResourceRetention};

pub trait Host {
    fn modules(&self) -> Vec<module::HostModule> {
        Vec::new()
    }
}
