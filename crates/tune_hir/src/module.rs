use crate::ModuleId;
use crate::item::Item;

#[derive(Debug, Clone)]
pub struct Module {
    pub id: ModuleId,
    pub items: Vec<Item>,
}
