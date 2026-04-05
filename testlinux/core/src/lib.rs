use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub id: String,
    pub title: String,
    pub completed: bool,
}

pub struct App {
    items: Vec<Item>,
}

impl App {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }
    
    pub fn add_item(&mut self, id: String, title: String) {
        self.items.push(Item {
            id,
            title,
            completed: false,
        });
    }
    
    pub fn toggle_item(&mut self, id: &str) {
        if let Some(item) = self.items.iter_mut().find(|i| i.id == id) {
            item.completed = !item.completed;
        }
    }
    
    pub fn delete_item(&mut self, id: &str) {
        self.items.retain(|i| i.id != id);
    }
    
    pub fn get_items(&self) -> &[Item] {
        &self.items
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}
