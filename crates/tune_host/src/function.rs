use tune_shape::Shape;

use crate::authority::Authority;

#[derive(Debug, Clone)]
pub struct HostFunction {
    pub name: String,
    pub params: Vec<HostParam>,
    pub ret: Shape,
    pub authorities: Vec<Authority>,
    pub task_safe: bool,
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
