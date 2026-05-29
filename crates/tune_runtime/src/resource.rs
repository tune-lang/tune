#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ResourceId(pub u64);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceHandle {
    pub id: ResourceId,
    pub type_name: String,
    pub task_safe: bool,
}

impl ResourceHandle {
    #[must_use]
    pub fn new(id: ResourceId, type_name: impl Into<String>) -> Self {
        Self {
            id,
            type_name: type_name.into(),
            task_safe: false,
        }
    }

    #[must_use]
    pub fn task_safe(mut self, task_safe: bool) -> Self {
        self.task_safe = task_safe;
        self
    }
}

pub trait Resource {
    fn close(&mut self) {}
}
