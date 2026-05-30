use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use crate::state::StateId;
use crate::value::{TaskSafetyError, Value};

#[derive(Debug, Clone)]
pub struct SequenceHandle {
    storage: Arc<Mutex<Vec<Value>>>,
}

impl SequenceHandle {
    #[must_use]
    pub fn new(values: Vec<Value>) -> Self {
        Self {
            storage: Arc::new(Mutex::new(values)),
        }
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.storage
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.storage
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .is_empty()
    }

    #[must_use]
    pub fn get(&self, index: usize) -> Option<Value> {
        self.storage
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(index)
            .cloned()
    }

    #[must_use]
    pub fn cloned_values(&self) -> Vec<Value> {
        self.storage
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    #[must_use]
    pub fn snapshot_values(&self) -> Vec<Value> {
        self.storage
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .iter()
            .map(Value::capture_snapshot)
            .collect()
    }

    #[must_use]
    pub fn task_safety_error(&self) -> Option<TaskSafetyError> {
        self.storage
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .iter()
            .find_map(Value::task_safety_error)
    }

    pub(crate) fn contains_state_inner(
        &self,
        state: StateId,
        visited: &mut HashSet<StateId>,
    ) -> bool {
        self.storage
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .iter()
            .any(|value| value.contains_state_inner(state, visited))
    }

    pub fn push_exclusive(&mut self, value: Value) {
        self.storage
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push(value);
    }

    pub fn push_shared_cow(&mut self, value: Value) {
        self.ensure_unique();
        self.push_exclusive(value);
    }

    pub fn set_exclusive(&mut self, index: usize, value: Value) -> Option<()> {
        let mut values = self
            .storage
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let slot = values.get_mut(index)?;
        *slot = value;
        Some(())
    }

    pub fn set_shared_cow(&mut self, index: usize, value: Value) -> Option<()> {
        self.ensure_unique();
        self.set_exclusive(index, value)
    }

    fn ensure_unique(&mut self) {
        if Arc::strong_count(&self.storage) <= 1 {
            return;
        }
        let values = self
            .storage
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();
        self.storage = Arc::new(Mutex::new(values));
    }
}

impl PartialEq for SequenceHandle {
    fn eq(&self, other: &Self) -> bool {
        if Arc::ptr_eq(&self.storage, &other.storage) {
            return true;
        }
        let left = self
            .storage
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let right = other
            .storage
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *left == *right
    }
}
