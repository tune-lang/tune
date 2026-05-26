use crate::Shape;

#[derive(Debug, Clone)]
pub struct BindingState {
    pub storage_shape: Shape,
    pub current_shape: Shape,
    pub is_literal_fact: bool,
}

#[derive(Debug, Default)]
pub struct StateFrame {
    pub bindings: Vec<BindingState>,
}
