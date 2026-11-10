use std::sync::Arc;

pub struct WebUserId(u64);
uniffi::custom_newtype!(WebUserId, u64);

pub struct WebEmail(String);
uniffi::custom_type!(WebEmail, String, {
    lower: |value| value.0,
    try_lift: |value| {
        if value.contains('@') { Ok(WebEmail(value.trim().to_lowercase())) }
        else { Err(uniffi::deps::anyhow::anyhow!("Email must contain @")) }
    },
});

pub type WebIp = std::net::Ipv4Addr;
uniffi::custom_type!(WebIp, String, {
    remote,
    lower: |value| value.to_string(),
    try_lift: |value| Ok(value.parse()?),
});

#[derive(uniffi::Record)]
pub struct WebCustomRecord {
    pub user_id: WebUserId,
    pub email: WebEmail,
    pub address: WebIp,
}

impl From<uniffi::UnexpectedUniFFICallbackError> for WebFailure {
    fn from(error: uniffi::UnexpectedUniFFICallbackError) -> Self {
        Self::InvalidInput {
            reason: format!("Unexpected callback failure: {error}"),
        }
    }
}

#[uniffi::export(callback_interface)]
pub trait WebFormatter: Send + Sync {
    fn on_count(&self, count: i32);
    fn format_profile(
        &self,
        profile: WebProfile,
        email: WebEmail,
    ) -> Result<WebProfile, WebFailure>;
}

#[uniffi::export(with_foreign)]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
pub trait WebService: Send + Sync {
    fn consume(
        self: std::sync::Arc<Self>,
        value: WebCustomRecord,
    ) -> Result<WebCustomRecord, WebFailure>;
    fn transform(&self, value: WebCustomRecord) -> Result<WebCustomRecord, WebFailure>;
    async fn transform_async(&self, value: WebCustomRecord) -> Result<WebCustomRecord, WebFailure>;
}

struct RustWebService;
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
impl WebService for RustWebService {
    fn consume(
        self: std::sync::Arc<Self>,
        value: WebCustomRecord,
    ) -> Result<WebCustomRecord, WebFailure> {
        self.transform(value)
    }
    fn transform(&self, value: WebCustomRecord) -> Result<WebCustomRecord, WebFailure> {
        if value.email.0 == "fail@example.com" {
            Err(WebFailure::InvalidInput {
                reason: "Rust service rejected email".into(),
            })
        } else {
            Ok(value)
        }
    }
    async fn transform_async(&self, value: WebCustomRecord) -> Result<WebCustomRecord, WebFailure> {
        self.transform(value)
    }
}

thread_local! {
    static RETAINED_FORMATTER: std::cell::RefCell<Option<std::sync::Arc<dyn WebFormatter>>> = const { std::cell::RefCell::new(None) };
}
#[derive(uniffi::Record)]
pub struct WebServiceRecord {
    pub service: std::sync::Arc<dyn WebService>,
    pub label: String,
}

#[uniffi::export]
impl Core {
    pub fn round_trip_custom(&self, value: WebCustomRecord) -> WebCustomRecord {
        value
    }
    pub fn round_trip_emails(&self, value: Vec<Option<WebEmail>>) -> Vec<Option<WebEmail>> {
        value
    }
    pub fn retain_formatter(&self, callback: Box<dyn WebFormatter>) {
        RETAINED_FORMATTER.with(|slot| *slot.borrow_mut() = Some(std::sync::Arc::from(callback)));
    }
    pub fn clear_formatter(&self) {
        RETAINED_FORMATTER.with(|slot| *slot.borrow_mut() = None);
    }
    pub fn use_retained_formatter(
        &self,
        profile: WebProfile,
        email: WebEmail,
    ) -> Result<WebProfile, WebFailure> {
        let callback = RETAINED_FORMATTER
            .with(|slot| slot.borrow().clone())
            .ok_or_else(|| WebFailure::InvalidInput {
                reason: "No formatter".into(),
            })?;
        callback.on_count(self.get_count());
        callback.format_profile(profile, email)
    }
    pub fn make_web_service(&self) -> std::sync::Arc<dyn WebService> {
        std::sync::Arc::new(RustWebService)
    }
    pub fn round_trip_service(
        &self,
        value: std::sync::Arc<dyn WebService>,
    ) -> std::sync::Arc<dyn WebService> {
        value
    }
    pub fn round_trip_service_record(&self, value: WebServiceRecord) -> WebServiceRecord {
        value
    }
    pub fn invoke_web_service(
        &self,
        service: std::sync::Arc<dyn WebService>,
        value: WebCustomRecord,
    ) -> Result<WebCustomRecord, WebFailure> {
        service.transform(value)
    }
    pub async fn invoke_web_service_async(
        &self,
        service: std::sync::Arc<dyn WebService>,
        value: WebCustomRecord,
    ) -> Result<WebCustomRecord, WebFailure> {
        service.transform_async(value).await
    }
}

#[cfg(test)]
mod callback_native_parity {
    use super::*;
    fn lift<
        T: uniffi::FfiConverter<crate::UniFfiTag>,
        B: uniffi::FfiConverter<crate::UniFfiTag>,
    >(
        value: B,
    ) -> uniffi::Result<T> {
        let mut bytes = Vec::new();
        B::write(value, &mut bytes);
        T::try_read(&mut bytes.as_slice())
    }
    fn custom() -> WebCustomRecord {
        WebCustomRecord {
            user_id: lift::<WebUserId, _>(u64::MAX).unwrap(),
            email: lift::<WebEmail, _>(" SAM@EXAMPLE.COM ".to_string()).unwrap(),
            address: lift::<WebIp, _>("127.0.0.1".to_string()).unwrap(),
        }
    }
    fn profile() -> WebProfile {
        WebProfile {
            display_name: "Samson".into(),
            scores: vec![1, 2],
            note: None,
            flags: HashMap::from([("local".into(), true)]),
            big_count: 12,
            choice: WebChoice::Empty,
            nested: vec![None],
        }
    }
    struct Formatter {
        counts: std::sync::Arc<std::sync::Mutex<Vec<i32>>>,
    }
    impl WebFormatter for Formatter {
        fn on_count(&self, count: i32) {
            self.counts.lock().unwrap().push(count);
        }
        fn format_profile(
            &self,
            mut value: WebProfile,
            email: WebEmail,
        ) -> Result<WebProfile, WebFailure> {
            assert_eq!(email.0, "sam@example.com");
            value.display_name += " via callback";
            Ok(value)
        }
    }
    fn ready<F: std::future::Future>(future: F) -> F::Output {
        struct Noop;
        impl std::task::Wake for Noop {
            fn wake(self: std::sync::Arc<Self>) {}
        }
        let waker = std::task::Waker::from(std::sync::Arc::new(Noop));
        match Box::pin(future)
            .as_mut()
            .poll(&mut std::task::Context::from_waker(&waker))
        {
            std::task::Poll::Ready(value) => value,
            std::task::Poll::Pending => panic!("Native fixture should complete immediately"),
        }
    }
    #[test]
    fn custom_converters_match_wasm_validation_and_normalization() {
        let core = Core::new();
        let output = core.round_trip_custom(custom());
        assert_eq!(output.user_id.0, u64::MAX);
        assert_eq!(output.email.0, "sam@example.com");
        assert_eq!(output.address.to_string(), "127.0.0.1");
        assert!(lift::<WebEmail, _>("invalid".to_string()).is_err());
        assert!(lift::<WebIp, _>("invalid".to_string()).is_err());
        let result = core.round_trip_emails(vec![
            None,
            Some(lift::<WebEmail, _>(" SAM@EXAMPLE.COM ".to_string()).unwrap()),
        ]);
        assert!(result[0].is_none());
        assert_eq!(result[1].as_ref().unwrap().0, "sam@example.com");
    }
    #[test]
    fn native_callback_retention_and_service_errors_match_wasm() {
        let core = Core::new();
        let counts = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        core.retain_formatter(Box::new(Formatter {
            counts: counts.clone(),
        }));
        let output = core
            .use_retained_formatter(profile(), custom().email)
            .unwrap();
        assert_eq!(output.display_name, "Samson via callback");
        assert_eq!(*counts.lock().unwrap(), vec![0]);
        core.clear_formatter();
        assert!(core
            .use_retained_formatter(profile(), custom().email)
            .is_err());
        let service = core.make_web_service();
        assert_eq!(
            service.transform(custom()).unwrap().email.0,
            "sam@example.com"
        );
        assert_eq!(
            ready(service.transform_async(custom())).unwrap().email.0,
            "sam@example.com"
        );
        let mut bad = custom();
        bad.email = lift::<WebEmail, _>("fail@example.com".to_string()).unwrap();
        assert!(service.transform(bad).is_err());
        let unexpected=<Result<WebProfile,WebFailure> as uniffi::LiftReturn<crate::UniFfiTag>>::handle_callback_unexpected_error(uniffi::UnexpectedUniFFICallbackError::new("unexpected JavaScript callback failure"));
        assert!(
            matches!(unexpected,Err(WebFailure::InvalidInput{reason})if reason.contains("Unexpected callback failure"))
        );
    }
}

pub struct WebLabel(String);
impl From<WebLabel> for String {
    fn from(value: WebLabel) -> Self {
        value.0
    }
}
impl TryFrom<String> for WebLabel {
    type Error = uniffi::deps::anyhow::Error;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.trim().is_empty() {
            Err(uniffi::deps::anyhow::anyhow!("Label cannot be empty"))
        } else {
            Ok(Self(value.trim().to_string()))
        }
    }
}
uniffi::custom_type!(WebLabel, String);
#[uniffi::export]
impl Core {
    pub fn round_trip_labels(&self, labels: Vec<WebLabel>) -> Vec<WebLabel> {
        labels
    }
}

#[uniffi::export]
impl Core {
    pub fn invoke_owned_service(
        &self,
        service: std::sync::Arc<dyn WebService>,
        value: WebCustomRecord,
    ) -> Result<WebCustomRecord, WebFailure> {
        service.consume(value)
    }
    pub fn owned_count(self: std::sync::Arc<Self>) -> i32 {
        self.get_count()
    }
}

#[uniffi::export(with_foreign)]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
#[cfg_attr(target_arch="wasm32",async_trait::async_trait(?Send))]
pub trait WebWide: Send + Sync {
    fn sum(&self, a: i32, b: i32, c: i32, d: i32, e: i32, f: i32, g: i32, h: i32, i: i32) -> i32;
    async fn sum_async(
        &self,
        a: i32,
        b: i32,
        c: i32,
        d: i32,
        e: i32,
        f: i32,
        g: i32,
        h: i32,
        i: i32,
    ) -> i32;
}
struct RustWide;
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
#[cfg_attr(target_arch="wasm32",async_trait::async_trait(?Send))]
impl WebWide for RustWide {
    fn sum(&self, a: i32, b: i32, c: i32, d: i32, e: i32, f: i32, g: i32, h: i32, i: i32) -> i32 {
        a + b + c + d + e + f + g + h + i
    }
    async fn sum_async(
        &self,
        a: i32,
        b: i32,
        c: i32,
        d: i32,
        e: i32,
        f: i32,
        g: i32,
        h: i32,
        i: i32,
    ) -> i32 {
        self.sum(a, b, c, d, e, f, g, h, i)
    }
}
#[uniffi::export]
impl Core {
    pub fn make_wide(&self) -> Arc<dyn WebWide> {
        Arc::new(RustWide)
    }
    pub fn invoke_wide(&self, callback: Arc<dyn WebWide>) -> i32 {
        callback.sum(1, 2, 3, 4, 5, 6, 7, 8, 9)
    }
    pub async fn invoke_wide_async(&self, callback: Arc<dyn WebWide>) -> i32 {
        callback.sum_async(1, 2, 3, 4, 5, 6, 7, 8, 9).await
    }
}
