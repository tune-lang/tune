use tune_shape::Shape;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostValueType {
    pub name: String,
    pub fields: Vec<HostValueField>,
}

impl HostValueType {
    #[must_use]
    pub fn new(name: impl Into<String>, fields: Vec<HostValueField>) -> Self {
        Self {
            name: name.into(),
            fields,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostValueField {
    pub name: String,
    pub shape: Shape,
}

impl HostValueField {
    #[must_use]
    pub fn new(name: impl Into<String>, shape: Shape) -> Self {
        Self {
            name: name.into(),
            shape,
        }
    }
}
