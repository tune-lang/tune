#[derive(Debug, Clone)]
pub struct CallableHead {
    pub name: Option<String>, // None represents `_` anonymous callable.
    pub params: Vec<String>,
}
