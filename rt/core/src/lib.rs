use uniffi;

#[derive(uniffi::Object)]
pub struct Core {}

#[uniffi::export]
impl Core {
    #[uniffi::constructor]
    pub fn new() -> Self {
        Self {}
    }

    pub fn greeting(&self) -> String {
        "Hello from JFFI".to_string()
    }
}

uniffi::setup_scaffolding!();
