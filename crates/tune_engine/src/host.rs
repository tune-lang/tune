use tune_host::Host;
use tune_host::HostExecutor;
use tune_host::module::HostModule;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EngineHostSymbolId(pub u32);

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
}
