uniffi::setup_scaffolding!();
pub struct Core;
impl Core {
    pub fn hello(&self) {}
}
pub trait ProgressSink: Send + Sync {
    fn on_progress(&self);
}
