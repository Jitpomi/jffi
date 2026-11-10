#[uniffi::export]
pub fn standalone_add(left: i32, right: i32) -> i32 { left + right }
#[uniffi::export]
pub async fn standalone_async(value: String) -> String { value }
#[uniffi::export]
pub fn standalone_profile(value: WebProfile) -> WebProfile { value }
#[uniffi::export]
pub fn standalone_custom(value: WebEmail) -> WebEmail { value }
#[uniffi::export]
pub fn standalone_failure() -> Result<i32, WebFailure> { Err(WebFailure::InvalidInput { reason: "standalone".into() }) }
#[cfg(target_arch = "wasm32")]
pub mod conditional_web {
    #[cfg_attr(target_arch = "wasm32", uniffi::export)]
    pub fn wasm_only() -> String { "wasm".into() }
    #[cfg(debug_assertions)]
    #[uniffi::export]
    pub fn debug_only() -> bool { true }
    #[cfg(not(debug_assertions))]
    #[uniffi::export]
    pub fn release_only() -> bool { true }
}
#[cfg(not(target_arch = "wasm32"))]
#[uniffi::export]
pub fn native_only() -> bool { true }
#[cfg(feature = "web-extra")]
#[uniffi::export]
pub fn feature_only() -> i32 { 42 }
#[derive(uniffi::Record)]
pub struct ConditionalRecord {
    pub common: String,
    #[cfg(target_arch = "wasm32")]
    pub browser: bool,
    #[cfg(not(target_arch = "wasm32"))]
    pub desktop: bool,
}
#[uniffi::export]
pub fn conditional_record(value: ConditionalRecord) -> ConditionalRecord { value }
