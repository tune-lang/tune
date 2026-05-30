use tune_host::Authority;
use tune_host::Host;
use tune_host::HostExecutor;
use tune_host::HostFunction;
use tune_host::HostResourceType;
pub use tune_host::HostSymbolId as EngineHostSymbolId;
use tune_host::HostValueType;
use tune_host::module::HostModule;
pub use tune_runtime::ResourceTypeId as EngineResourceTypeId;

use crate::Tune;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineHostSymbol {
    pub id: EngineHostSymbolId,
    pub module: String,
    pub function: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EngineHostResourceType {
    pub id: EngineResourceTypeId,
    pub module: String,
    pub resource: HostResourceType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineHostValueType {
    pub module: String,
    pub value: HostValueType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HostRegistration {
    pub module_count: usize,
    pub function_count: usize,
    pub value_count: usize,
    pub resource_count: usize,
}

#[derive(Default)]
pub(crate) struct HostRegistry {
    modules: Vec<HostModule>,
    symbols: Vec<EngineHostSymbol>,
    values: Vec<EngineHostValueType>,
    resources: Vec<EngineHostResourceType>,
    executors: Vec<Option<HostExecutor>>,
    authorities: Vec<Vec<Authority>>,
}

impl HostRegistry {
    pub(crate) fn register(&mut self, host: &impl Host) -> HostRegistration {
        let modules = host.modules();
        let module_count = modules.len();
        let function_count = modules
            .iter()
            .map(|module| module.functions.len())
            .sum::<usize>();
        let value_count = modules
            .iter()
            .map(|module| module.values.len())
            .sum::<usize>();
        let resource_count = modules
            .iter()
            .map(|module| module.resources.len())
            .sum::<usize>();

        for module in &modules {
            for value in &module.values {
                self.values.push(EngineHostValueType {
                    module: module.name.clone(),
                    value: value.clone(),
                });
            }

            for resource in &module.resources {
                let id =
                    EngineResourceTypeId(u32::try_from(self.resources.len()).unwrap_or(u32::MAX));
                self.resources.push(EngineHostResourceType {
                    id,
                    module: module.name.clone(),
                    resource: resource.clone(),
                });
            }

            for function in &module.functions {
                let id = EngineHostSymbolId(u32::try_from(self.symbols.len()).unwrap_or(u32::MAX));
                self.symbols.push(EngineHostSymbol {
                    id,
                    module: module.name.clone(),
                    function: function.name.clone(),
                });
                self.executors.push(function.executor.clone());
                self.authorities.push(function.authorities.clone());
            }
        }

        self.modules.extend(modules);
        HostRegistration {
            module_count,
            function_count,
            value_count,
            resource_count,
        }
    }

    pub(crate) fn modules(&self) -> &[HostModule] {
        &self.modules
    }

    pub(crate) fn symbols(&self) -> &[EngineHostSymbol] {
        &self.symbols
    }

    pub(crate) fn resources(&self) -> &[EngineHostResourceType] {
        &self.resources
    }

    pub(crate) fn values(&self) -> &[EngineHostValueType] {
        &self.values
    }

    pub(crate) fn resource(&self, id: EngineResourceTypeId) -> Option<&EngineHostResourceType> {
        self.resources
            .get(id.0 as usize)
            .filter(|resource| resource.id == id)
    }

    pub(crate) fn symbol(&self, id: EngineHostSymbolId) -> Option<&EngineHostSymbol> {
        self.symbols.get(id.0 as usize)
    }

    pub(crate) fn executors(&self) -> Vec<Option<HostExecutor>> {
        self.executors.clone()
    }

    pub(crate) fn authorities(&self) -> Vec<Vec<Authority>> {
        self.authorities.clone()
    }

    pub(crate) fn vm_resource_types(&self) -> Vec<tune_vm::VmHostResourceType> {
        self.resources
            .iter()
            .map(|resource| {
                tune_vm::VmHostResourceType::new(
                    resource.id,
                    format!("{}.{}", resource.module, resource.resource.name),
                )
                .task_safe(resource.resource.task_safe)
                .retention(resource.resource.retention.clone())
                .cleanup(resource.resource.cleanup.clone())
                .with_authorities(resource.resource.authorities.clone())
                .with_cleanup_executor_if_present(resource.resource.cleanup_executor.clone())
            })
            .collect()
    }

    pub(crate) fn function(
        &self,
        module_name: &str,
        function_name: &str,
    ) -> Option<(EngineHostSymbolId, &HostFunction)> {
        for module in &self.modules {
            if module.name != module_name {
                continue;
            }
            let function = module
                .functions
                .iter()
                .find(|function| function.name == function_name)?;
            let symbol = self
                .symbols
                .iter()
                .find(|symbol| symbol.module == module_name && symbol.function == function_name)?;
            return Some((symbol.id, function));
        }
        None
    }
}

impl Tune {
    #[must_use]
    pub fn with_host(mut self, host: &impl Host) -> Self {
        self.register_host(host);
        self
    }

    #[must_use]
    pub fn with_std(mut self) -> Self {
        self.register_std();
        self
    }
}
