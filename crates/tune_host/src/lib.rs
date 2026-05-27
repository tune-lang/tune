pub mod authority;
pub mod function;
pub mod module;
pub mod resource;

pub use authority::Authority;
pub use function::{HostFunction, HostParam};
pub use module::HostModule;
pub use resource::HostResourceType;

pub trait Host {
    fn modules(&self) -> Vec<module::HostModule> {
        Vec::new()
    }
}
