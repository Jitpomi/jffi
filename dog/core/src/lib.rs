use std::sync::Mutex;
use uniffi;

#[derive(uniffi::Object)]
pub struct Core {
    name: Mutex<String>,
}

#[uniffi::export]
impl Core {
    #[uniffi::constructor]
    pub fn new() -> Self {
        Self {
            name: Mutex::new("Rust".to_string()),
        }
    }
    
    pub fn set_name(&self, name: String) {
        let mut name_lock = self.name.lock().unwrap();
        name_lock.clone_from(&name);
    }

    pub fn get_name(&self) -> String {
        let name_lock = self.name.lock().unwrap();
        name_lock.clone() // the guard drops and releases the lock.
    }
}

uniffi::setup_scaffolding!();
