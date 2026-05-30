use std::any::Any;
use std::fmt;
use std::sync::Arc;

use tune_shape::Shape;

use crate::authority::Authority;
use tune_runtime::{ResourceHandle, Value};

pub type HostCallResult = Result<Value, HostCallError>;
pub type HostResourceObject = Arc<dyn Any + Send + Sync>;

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

    fn call_with_context(&self, args: &[Value], _context: &dyn HostContext) -> HostCallResult {
        self.call(args)
    }
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

struct ContextHostCallable<F> {
    callable: F,
}

impl<F> HostCallable for ContextHostCallable<F>
where
    F: Fn(&[Value], &dyn HostContext) -> HostCallResult + Send + Sync,
{
    fn call(&self, _args: &[Value]) -> HostCallResult {
        Err(HostCallError::new(
            "host function requires an execution context",
        ))
    }

    fn call_with_context(&self, args: &[Value], context: &dyn HostContext) -> HostCallResult {
        (self.callable)(args, context)
    }
}

impl HostExecutor {
    #[must_use]
    pub fn new(callable: impl HostCallable + 'static) -> Self {
        Self {
            callable: Arc::new(callable),
        }
    }

    #[must_use]
    pub fn new_with_context(
        callable: impl Fn(&[Value], &dyn HostContext) -> HostCallResult + Send + Sync + 'static,
    ) -> Self {
        Self {
            callable: Arc::new(ContextHostCallable { callable }),
        }
    }

    pub fn call(&self, args: &[Value]) -> HostCallResult {
        self.callable.call(args)
    }

    pub fn call_with_context(&self, args: &[Value], context: &dyn HostContext) -> HostCallResult {
        self.callable.call_with_context(args, context)
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
    pub doc: Option<String>,
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
            doc: None,
            params,
            ret,
            authorities: Vec::new(),
            task_safe: false,
            executor: None,
        }
    }

    #[must_use]
    pub fn with_doc(mut self, doc: impl Into<String>) -> Self {
        self.doc = Some(doc.into());
        self
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

    #[must_use]
    pub fn with_context_executor(
        mut self,
        executor: impl Fn(&[Value], &dyn HostContext) -> HostCallResult + Send + Sync + 'static,
    ) -> Self {
        self.executor = Some(HostExecutor::new_with_context(executor));
        self
    }
}

#[derive(Debug, Clone)]
pub struct HostParam {
    pub name: String,
    pub shape: Shape,
}

pub trait HostContext {
    fn insert_resource(
        &self,
        type_name: &str,
        object: HostResourceObject,
    ) -> Result<ResourceHandle, HostCallError>;

    fn get_resource(&self, handle: &ResourceHandle) -> Result<HostResourceObject, HostCallError>;

    fn close_resource(&self, handle: &ResourceHandle) -> Result<(), HostCallError>;
}

pub fn downcast_resource<T>(object: HostResourceObject) -> Result<Arc<T>, HostCallError>
where
    T: Any + Send + Sync,
{
    object
        .downcast::<T>()
        .map_err(|_| HostCallError::new("host resource object has unexpected type"))
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
