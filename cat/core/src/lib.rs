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
            name: Mutex::new("JFFI".to_string()),
        }
    }

    pub fn greeting(&self) -> String {
        let name_lock = self.name.lock().unwrap();
        format!("Hello from {}", name_lock.clone())
    }
    
    pub fn set_name(&self, name: String) {
        let mut name_lock = self.name.lock().unwrap();
        *name_lock = name;
    }
}

uniffi::setup_scaffolding!();
