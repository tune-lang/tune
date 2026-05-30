use std::fmt;
use std::sync::Arc;

use tune_shape::Shape;

use crate::authority::Authority;
use tune_runtime::Value;

pub type HostCallResult = Result<Value, HostCallError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostCallError {
    pub message: String,
}

impl HostCallError {
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

pub trait HostCallable: Send + Sync {
    fn call(&self, args: &[Value]) -> HostCallResult;
}

impl<F> HostCallable for F
where
    F: Fn(&[Value]) -> HostCallResult + Send + Sync,
{
    fn call(&self, args: &[Value]) -> HostCallResult {
        self(args)
    }
}

#[derive(Clone)]
pub struct HostExecutor {
    callable: Arc<dyn HostCallable>,
}

impl HostExecutor {
    #[must_use]
    pub fn new(callable: impl HostCallable + 'static) -> Self {
        Self {
            callable: Arc::new(callable),
        }
    }

    pub fn call(&self, args: &[Value]) -> HostCallResult {
        self.callable.call(args)
    }
}

impl fmt::Debug for HostExecutor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HostExecutor").finish_non_exhaustive()
    }
}

#[derive(Debug, Clone)]
pub struct HostFunction {
    pub name: String,
    pub params: Vec<HostParam>,
    pub ret: Shape,
    pub authorities: Vec<Authority>,
    pub task_safe: bool,
    /// `None` means declaration-only: the compiler may type-check against this host function,
    /// but VM execution must report a missing executor if the function is called.
    pub executor: Option<HostExecutor>,
}

impl HostFunction {
    #[must_use]
    pub fn new(name: impl Into<String>, params: Vec<HostParam>, ret: Shape) -> Self {
        Self {
            name: name.into(),
            params,
            ret,
            authorities: Vec::new(),
            task_safe: false,
            executor: None,
        }
    }

    #[must_use]
    pub fn with_authorities(mut self, authorities: Vec<Authority>) -> Self {
        self.authorities = authorities;
        self
    }

    #[must_use]
    pub fn task_safe(mut self, task_safe: bool) -> Self {
        self.task_safe = task_safe;
        self
    }

    #[must_use]
    pub fn with_executor(mut self, executor: impl HostCallable + 'static) -> Self {
        self.executor = Some(HostExecutor::new(executor));
        self
    }
}

#[derive(Debug, Clone)]
pub struct HostParam {
    pub name: String,
    pub shape: Shape,
}

impl HostParam {
    #[must_use]
    pub fn new(name: impl Into<String>, shape: Shape) -> Self {
        Self {
            name: name.into(),
            shape,
        }
    }
}
