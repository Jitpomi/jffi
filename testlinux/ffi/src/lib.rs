use testlinux_core::{App, Item};
use std::sync::Mutex;

#[derive(uniffi::Record)]
pub struct ItemViewModel {
    pub id: String,
    pub title: String,
    pub completed: bool,
}

impl From<&Item> for ItemViewModel {
    fn from(item: &Item) -> Self {
        Self {
            id: item.id.clone(),
            title: item.title.clone(),
            completed: item.completed,
        }
    }
}

#[derive(uniffi::Object)]
pub struct FfiApp {
    app: Mutex<App>,
}

#[uniffi::export]
impl FfiApp {
    #[uniffi::constructor]
    pub fn new() -> Self {
        Self {
            app: Mutex::new(App::new()),
        }
    }
    
    pub fn add_item(&self, id: String, title: String) -> Vec<ItemViewModel> {
        let mut app = self.app.lock().unwrap();
        app.add_item(id, title);
        app.get_items().iter().map(ItemViewModel::from).collect()
    }
    
    pub fn toggle_item(&self, id: String) -> Vec<ItemViewModel> {
        let mut app = self.app.lock().unwrap();
        app.toggle_item(&id);
        app.get_items().iter().map(ItemViewModel::from).collect()
    }
    
    pub fn delete_item(&self, id: String) -> Vec<ItemViewModel> {
        let mut app = self.app.lock().unwrap();
        app.delete_item(&id);
        app.get_items().iter().map(ItemViewModel::from).collect()
    }
    
    pub fn get_items(&self) -> Vec<ItemViewModel> {
        let app = self.app.lock().unwrap();
        app.get_items().iter().map(ItemViewModel::from).collect()
    }
}

uniffi::setup_scaffolding!();
