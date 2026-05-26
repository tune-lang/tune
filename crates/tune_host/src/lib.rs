pub mod authority;
pub mod function;
pub mod module;
pub mod resource;

pub trait Host {
    fn modules(&self) -> Vec<module::HostModule> {
        Vec::new()
    }
}
