// Deliberately absent from Core's public Rust namespace. Legacy UniFFI metadata
// contains ABI names but not the Rust module path needed by a direct-call adapter.
mod inaccessible {
    static LIVE:std::sync::atomic::AtomicI32=std::sync::atomic::AtomicI32::new(0);
    #[uniffi::export] fn private_live_count()->i32 {LIVE.load(std::sync::atomic::Ordering::SeqCst)}
    #[derive(uniffi::Record)]
    struct PrivateProfile { label:String, count:i32 }
    #[uniffi::export]
    fn private_round_trip(value:PrivateProfile)->PrivateProfile { value }
    #[derive(uniffi::Object)]
    struct PrivateCounter { value:std::sync::Mutex<i32> }
    #[uniffi::export]
    impl PrivateCounter {
        #[uniffi::constructor] fn new(value:i32)->Self {LIVE.fetch_add(1,std::sync::atomic::Ordering::SeqCst);Self{value:std::sync::Mutex::new(value)}}
        fn increment(&self)->i32 {let mut v=self.value.lock().unwrap();*v+=1;*v}
        async fn async_value(&self)->i32 {*self.value.lock().unwrap()}
    }
    impl Drop for PrivateCounter {fn drop(&mut self){LIVE.fetch_sub(1,std::sync::atomic::Ordering::SeqCst);}}
    #[uniffi::export] fn private_counter_value(counter:std::sync::Arc<PrivateCounter>,invalid_after_counter:Vec<i32>)->i32 {let _=invalid_after_counter;let value=*counter.value.lock().unwrap();value}
    #[uniffi::export(callback_interface)]
    trait PrivateListener:Send+Sync {fn transform(&self,value:i32)->i32;}
    #[uniffi::export] fn private_callback(value:Box<dyn PrivateListener>)->i32 {value.transform(42)}
    #[derive(Clone,Debug)] struct PrivateId(u64);
    uniffi::custom_newtype!(PrivateId,u64);
    #[uniffi::export] fn private_id(value:PrivateId)->PrivateId {value}
}
// This macro lives in a dependency, so scanning Core's macro_rules cannot
// discover its output. It uses repetition and identifier parameters as well.
shared_alias::legacy_export!{GeneratedPrivateRecord, generated_private_round_trip, first, second}
mod private_cancellation {
    #[uniffi::export(callback_interface)]
    #[cfg_attr(not(target_arch="wasm32"),async_trait::async_trait)]
    #[cfg_attr(target_arch="wasm32",async_trait::async_trait(?Send))]
    trait PendingListener:Send+Sync {async fn wait(&self)->i32;}
    #[uniffi::export]
    async fn private_cancel_callback(callback:Box<dyn PendingListener>)->bool {
        let mut future=Box::pin(callback.wait());
        std::future::poll_fn(|cx|{let _=std::future::Future::poll(future.as_mut(),cx);std::task::Poll::Ready(())}).await;
        drop(future);
        true
    }
}
