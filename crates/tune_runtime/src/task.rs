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

#[derive(Debug, Clone, PartialEq)]
pub enum TaskJoinOutcome {
    Ready(Value),
    Pending(TaskId),
    UnrecoverablePanic(TunePanic),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskExecutionMode {
    /// Evaluate spawned work at the point of spawn.
    Immediate,
    /// Defer spawned work until join.
    DeferredUntilJoin,
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

    #[must_use]
    pub fn join(self) -> TaskJoinOutcome {
        match self.state {
            TaskState::Pending => TaskJoinOutcome::Pending(self.id),
            TaskState::Ready(value) => TaskJoinOutcome::Ready(value),
            TaskState::Panicked(panic) => TaskJoinOutcome::UnrecoverablePanic(panic),
        }
    }
}
