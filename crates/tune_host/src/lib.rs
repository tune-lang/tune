pub mod authority;
pub mod function;
pub mod module;
pub mod resource;
pub mod symbol;

pub use authority::Authority;
pub use function::{HostCallError, HostCallResult, HostExecutor, HostFunction, HostParam};
pub use module::HostModule;
pub use resource::{HostResourceType, ResourceCleanup, ResourceRetention};
pub use symbol::HostSymbolId;

pub trait Host {
    fn modules(&self) -> Vec<module::HostModule> {
        Vec::new()
    }
}
