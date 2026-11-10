use std::time::{Duration, SystemTime};
#[derive(uniffi::Record)]
pub struct TimeValues { pub timestamp: SystemTime, pub duration: Duration }
#[uniffi::export]
pub fn round_trip_time(value: TimeValues) -> TimeValues { value }
#[uniffi::export]
pub fn borrow_profile(value: &WebProfile) -> String { value.display_name.clone() }
#[uniffi::export]
pub fn compatibility_values() -> Vec<String> {
    vec![i64::MIN.to_string(),u64::MAX.to_string(),"Hello 🌍".into()]
}
macro_rules! export_api {
    () => {
        #[derive(uniffi::Record)] pub struct MacroRecord { pub value: String }
        #[uniffi::export] pub fn macro_round_trip(value: MacroRecord) -> MacroRecord {value}
    };
}
export_api!();
include!(concat!(env!("CARGO_MANIFEST_DIR"),"/src/web_included.rs"));
#[derive(uniffi::Object)]
pub struct AsyncFactory {value: i32}
#[uniffi::export]
impl AsyncFactory {
    #[uniffi::constructor]
    pub async fn new(value:i32)->Self {Self{value}}
    pub fn value(&self)->i32 {self.value}
}
#[derive(uniffi::Object)]
pub struct FactoryOnly {value:i32}
#[uniffi::export]
impl FactoryOnly {
    #[uniffi::constructor]
    pub fn create(value:i32)->Self {Self{value}}
    pub fn value(&self)->i32 {self.value}
}
#[cfg(test)] mod compatibility_tests {
    use super::*;
    #[test] fn time_boundaries_and_generated_apis() {
        let nanos=Duration::new(9_007_199_254,987_654_321);
        let time=std::time::UNIX_EPOCH.checked_sub(nanos).unwrap();
        let result=round_trip_time(TimeValues{timestamp:time,duration:nanos});
        assert_eq!(result.timestamp,time); assert_eq!(result.duration,nanos);
        assert_eq!(macro_round_trip(MacroRecord{value:"macro".into()}).value,"macro");
        assert_eq!(included_value(),73);
        assert_eq!(FactoryOnly::create(31).value(),31);
    }
}

#[uniffi::export]
pub fn round_trip_bytes(value:Vec<u8>)->Vec<u8>{value}

pub use shared_alias::SharedRecord as ImportedProfile;
#[uniffi::export]
pub fn external_profile(value: ImportedProfile)->ImportedProfile{value}
mod hidden {
    #[derive(uniffi::Record)] pub struct HiddenRecord { pub message:String }
    #[uniffi::export] pub fn hidden_round_trip(value:HiddenRecord)->HiddenRecord{value}
}
pub use self::hidden::{HiddenRecord as PublicAlias, hidden_round_trip as public_alias_function};
mod wildcard {
    #[derive(uniffi::Record)] pub struct GlobRecord { pub value:bool }
    #[uniffi::export] pub fn glob_round_trip(value:GlobRecord)->GlobRecord{value}
}
pub use self::wildcard::*;

include!(concat!(env!("OUT_DIR"),"/web_generated.rs"));

macro_rules! named_record { ($name:ident) => { #[derive(uniffi::Record)] pub struct $name { pub label:String } }; }
named_record!(NamedMacroRecord);
#[uniffi::export] pub fn named_macro_round_trip(value:NamedMacroRecord)->NamedMacroRecord{value}
#[derive(uniffi::Record)]
#[cfg_attr(feature="compat-renames",uniffi(name="PublicRecord"))]
pub struct RustRecord {pub label:String}
#[cfg_attr(feature="compat-renames",uniffi::export(name="public_record"))]
#[cfg_attr(not(feature="compat-renames"),uniffi::export)]
pub fn rust_record(value:RustRecord)->RustRecord{value}
#[derive(uniffi::Object)]
#[cfg_attr(feature="compat-renames",uniffi(name="PublicObject"))]
pub struct RustNamed {value:i32}
#[cfg_attr(feature="compat-renames",uniffi::export(name="PublicObject"))]
#[cfg_attr(not(feature="compat-renames"),uniffi::export)]
impl RustNamed {
    #[uniffi::constructor] pub fn new(value:i32)->Self{Self{value}}
    #[cfg_attr(feature="compat-renames",uniffi::method(name="read_value"))]
    pub fn actual_value(&self)->i32{self.value}
}
