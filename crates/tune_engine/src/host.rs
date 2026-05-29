use tune_host::Authority;
use tune_host::Host;
use tune_host::HostExecutor;
use tune_host::HostFunction;
pub use tune_host::HostSymbolId as EngineHostSymbolId;
use tune_host::module::HostModule;

use crate::Tune;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineHostSymbol {
    pub id: EngineHostSymbolId,
    pub module: String,
    pub function: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HostRegistration {
    pub module_count: usize,
    pub function_count: usize,
}

#[derive(Default)]
pub(crate) struct HostRegistry {
    modules: Vec<HostModule>,
    symbols: Vec<EngineHostSymbol>,
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

        for module in &modules {
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
        }
    }

    pub(crate) fn modules(&self) -> &[HostModule] {
        &self.modules
    }

    pub(crate) fn symbols(&self) -> &[EngineHostSymbol] {
        &self.symbols
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
