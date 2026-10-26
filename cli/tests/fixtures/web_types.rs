use super::Core;
use std::collections::HashMap;

#[derive(Clone, Debug, uniffi::Enum)]
pub enum WebChoice {
    Empty,
    Named { display_label: String, weight: i32 },
}
#[derive(Clone, Debug, uniffi::Record)]
pub struct WebProfile {
    pub display_name: String,
    pub scores: Vec<i32>,
    pub note: Option<String>,
    pub flags: HashMap<String, bool>,
    pub big_count: u64,
    pub choice: WebChoice,
    pub nested: Vec<Option<WebChoice>>,
}
#[derive(Debug, uniffi::Error)]
pub enum WebFailure {
    InvalidInput { reason: String },
}
impl std::fmt::Display for WebFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Invalid web input")
    }
}
impl std::error::Error for WebFailure {}
#[uniffi::export]
impl Core {
    pub fn round_trip_profile(&self, value: WebProfile) -> WebProfile {
        value
    }
    pub fn optional_profile(&self, value: Option<WebProfile>) -> Option<WebProfile> {
        value
    }
    pub fn round_trip_choices(&self, value: Vec<WebChoice>) -> Vec<WebChoice> {
        value
    }
    pub fn validate_profile(&self, value: WebProfile) -> Result<WebProfile, WebFailure> {
        if value.display_name.is_empty() {
            Err(WebFailure::InvalidInput {
                reason: "Name is required".into(),
            })
        } else {
            Ok(value)
        }
    }
    pub async fn async_profile(&self, value: WebProfile) -> Result<WebProfile, WebFailure> {
        self.validate_profile(value)
    }
}

#[derive(uniffi::Object)]
pub struct WebCounter {
    count: std::sync::Mutex<i32>,
}
#[uniffi::export]
impl WebCounter {
    #[uniffi::constructor]
    pub fn new(initial_count: i32) -> Self {
        Self {
            count: std::sync::Mutex::new(initial_count),
        }
    }
    pub fn increment(&self) -> i32 {
        let mut count = self.count.lock().unwrap();
        *count += 1;
        *count
    }
    pub fn get_count(&self) -> i32 {
        *self.count.lock().unwrap()
    }
}
#[derive(uniffi::Record)]
pub struct WebObjectEnvelope {
    pub counter: std::sync::Arc<WebCounter>,
    pub label: String,
}
#[uniffi::export]
impl Core {
    pub fn make_web_counter(&self) -> std::sync::Arc<WebCounter> {
        std::sync::Arc::new(WebCounter::new(0))
    }
    pub fn round_trip_object(
        &self,
        counter: std::sync::Arc<WebCounter>,
    ) -> std::sync::Arc<WebCounter> {
        counter
    }
    pub fn round_trip_object_envelope(&self, envelope: WebObjectEnvelope) -> WebObjectEnvelope {
        envelope
    }
}
