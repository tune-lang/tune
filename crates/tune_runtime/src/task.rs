use crate::panic::TunePanic;
use crate::value::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TaskId(pub u64);

#[derive(Debug, Clone, PartialEq)]
pub struct Task {
    pub id: TaskId,
    pub state: TaskState,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TaskState {
    Pending,
    Ready(Value),
    Panicked(TunePanic),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskJoinError {
    Pending(TaskId),
    Panicked(TunePanic),
}

impl Task {
    #[must_use]
    pub fn pending(id: TaskId) -> Self {
        Self {
            id,
            state: TaskState::Pending,
        }
    }

    #[must_use]
    pub fn ready(id: TaskId, value: Value) -> Self {
        Self {
            id,
            state: TaskState::Ready(value),
        }
    }

    #[must_use]
    pub fn panicked(id: TaskId, panic: TunePanic) -> Self {
        Self {
            id,
            state: TaskState::Panicked(panic),
        }
    }

    pub fn join(self) -> Result<Value, TaskJoinError> {
        match self.state {
            TaskState::Pending => Err(TaskJoinError::Pending(self.id)),
            TaskState::Ready(value) => Ok(value),
            TaskState::Panicked(panic) => Err(TaskJoinError::Panicked(panic)),
        }
    }
}
