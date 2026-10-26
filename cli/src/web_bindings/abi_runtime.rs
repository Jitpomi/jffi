// Included in the generated WASM adapter, never compiled into the CLI.
use serde_json::Value;
use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    future::Future,
    pin::Pin,
    task::{Context, Poll, Waker},
};
use wasm_bindgen::{prelude::*, JsCast};
type Status = uniffi::RustCallStatus;
thread_local! {
    static TYPES:Value=serde_json::from_str(JFFI_ABI_SCHEMA).expect("generated ABI schema");
    static OBJECTS:RefCell<BTreeMap<u32,(usize,u64)>>=RefCell::new(BTreeMap::new());
    static CALLBACKS:RefCell<BTreeMap<u64,(usize,JsValue,usize)>>=RefCell::new(BTreeMap::new());
    static NEXT:Cell<u64>=const{Cell::new(1)};
    static FUTURES:RefCell<BTreeMap<u64,(bool,Waker)>>=RefCell::new(BTreeMap::new());
    static TASKS:RefCell<BTreeMap<u64,Task>>=RefCell::new(BTreeMap::new());
}
fn next() -> u64 {
    NEXT.with(|c| {
        let n = c.get();
        c.set(n.checked_add(2).expect("ABI handle limit reached"));
        n
    })
}
fn ty(id: usize) -> Value {
    TYPES.with(|t| t[id].clone())
}
fn index(v: &Value, key: &str) -> usize {
    v[key].as_u64().unwrap() as usize
}
fn kind(v: &Value) -> &str {
    v["kind"].as_str().unwrap()
}
fn builtin(mut id: usize) -> usize {
    loop {
        let t = ty(id);
        if kind(&t) != "custom" {
            return id;
        }
        id = index(&t, "inner");
    }
}
fn type_error(msg: impl AsRef<str>) -> JsValue {
    js_sys::TypeError::new(msg.as_ref()).into()
}
fn get(v: &JsValue, k: &str) -> Result<JsValue, JsValue> {
    js_sys::Reflect::get(v, &JsValue::from_str(k))
}
fn put(v: &JsValue, k: &str, value: &JsValue) -> Result<(), JsValue> {
    js_sys::Reflect::set(v, &JsValue::from_str(k), value).map(|_| ())
}
fn text(v: &JsValue) -> Result<String, JsValue> {
    v.as_string().ok_or_else(|| type_error("Expected a string"))
}
fn int(v: &JsValue, bits: u32, signed: bool) -> Result<i128, JsValue> {
    let n = if bits == 64 {
        if !v.is_bigint() {
            return Err(type_error("Expected a bigint"));
        }
        v.unchecked_ref::<js_sys::BigInt>()
            .to_string(10)
            .map_err(JsValue::from)?
            .as_string()
            .unwrap()
            .parse::<i128>()
            .map_err(|_| type_error("Integer outside exported range"))?
    } else {
        let n = v
            .as_f64()
            .filter(|n| n.is_finite() && n.fract() == 0.0)
            .ok_or_else(|| type_error("Expected an integer"))?;
        n as i128
    };
    let (min, max) = if signed {
        (-(1i128 << (bits - 1)), (1i128 << (bits - 1)) - 1)
    } else {
        (0, (1i128 << bits) - 1)
    };
    if n < min || n > max {
        return Err(type_error("Integer outside exported range"));
    }
    Ok(n)
}
fn big(n: i128) -> JsValue {
    js_sys::BigInt::new(&JsValue::from_str(&n.to_string()))
        .unwrap()
        .into()
}
fn big_input(v: &JsValue) -> Result<i128, JsValue> {
    if !v.is_bigint() {
        return Err(type_error("Expected nanoseconds as bigint"));
    }
    v.unchecked_ref::<js_sys::BigInt>()
        .to_string(10)
        .map_err(JsValue::from)?
        .as_string()
        .unwrap()
        .parse()
        .map_err(|_| type_error("Nanoseconds outside supported range"))
}
struct Transaction {
    handles: Vec<(usize, u64)>,
    callbacks: Vec<u64>,
    committed: bool,
}
impl Transaction {
    fn new() -> Self {
        Self {
            handles: vec![],
            callbacks: vec![],
            committed: false,
        }
    }
    fn commit(&mut self) {
        self.committed = true;
    }
}
impl Drop for Transaction {
    fn drop(&mut self) {
        if !self.committed {
            for (i, h) in self.handles.drain(..) {
                drop_handle(i, h)
            }
            for h in self.callbacks.drain(..) {
                callback_free(h)
            }
        }
    }
}
fn callback_register(object: usize, v: &JsValue, tx: &mut Transaction) -> Result<u64, JsValue> {
    if !v.is_object() {
        return Err(type_error("Expected a callback object"));
    }
    for name in callback_methods(object) {
        if !get(v, name)?.is_function() {
            return Err(type_error(format!("Missing callback method {name}")));
        }
    }
    let id = next();
    CALLBACKS.with(|r| r.borrow_mut().insert(id, (object, v.clone(), 1)));
    tx.callbacks.push(id);
    Ok(id)
}
extern "C" fn callback_free(id: u64) {
    CALLBACKS.with(|r| {
        let mut r = r.borrow_mut();
        if let Some((_, _, refs)) = r.get_mut(&id) {
            *refs -= 1;
            if *refs == 0 {
                r.remove(&id);
            }
        }
    });
}
extern "C" fn callback_clone(id: u64) -> u64 {
    CALLBACKS.with(|r| {
        if let Some((_, _, refs)) = r.borrow_mut().get_mut(&id) {
            *refs = refs.checked_add(1).expect("Callback reference limit");
        }
    });
    id
}
fn callback_value(id: u64) -> Result<JsValue, JsValue> {
    CALLBACKS
        .with(|r| r.borrow().get(&id).map(|(_, v, _)| v.clone()))
        .ok_or_else(|| type_error("Callback has been freed"))
}
fn object_input(t: &Value, v: &JsValue, tx: &mut Transaction) -> Result<u64, JsValue> {
    let object = index(t, "object");
    if kind(t) == "callback" && t["foreign"] == true {
        return callback_register(object, v, tx);
    }
    let id = get(v, "__jffiHandle")
        .map_err(|_| type_error("Object has been freed"))?
        .as_f64()
        .filter(|n| *n > 0.0 && *n <= u32::MAX as f64 && n.fract() == 0.0)
        .ok_or_else(|| type_error("Expected a live exported object"))? as u32;
    let (kind, h) = OBJECTS
        .with(|r| r.borrow().get(&id).copied())
        .ok_or_else(|| type_error("Object has been freed"))?;
    if kind != object {
        return Err(type_error("Incorrect exported object type"));
    }
    let h = clone_handle(object, h)?;
    tx.handles.push((object, h));
    Ok(h)
}
fn array(v: &JsValue) -> Result<js_sys::Array, JsValue> {
    if !js_sys::Array::is_array(v) {
        return Err(type_error("Expected an array"));
    }
    Ok(v.clone().unchecked_into())
}
fn write(id: usize, v: &JsValue, out: &mut Vec<u8>, tx: &mut Transaction) -> Result<(), JsValue> {
    let t = ty(id);
    match kind(&t) {
        "custom" => write(index(&t, "inner"), v, out, tx)?,
        "void" => {}
        "bool" => out.push(
            v.as_bool()
                .ok_or_else(|| type_error("Expected a boolean"))? as u8,
        ),
        k @ ("u8" | "i8" | "u16" | "i16" | "u32" | "i32" | "u64" | "i64") => {
            let bits = k[1..].parse::<u32>().unwrap();
            let n = int(v, bits, k.starts_with('i'))?;
            out.extend_from_slice(&n.to_be_bytes()[16 - (bits / 8) as usize..]);
        }
        "f32" => out.extend_from_slice(
            &(v.as_f64().ok_or_else(|| type_error("Expected a number"))? as f32).to_be_bytes(),
        ),
        "f64" => out.extend_from_slice(
            &v.as_f64()
                .ok_or_else(|| type_error("Expected a number"))?
                .to_be_bytes(),
        ),
        "string" => {
            let s = text(v)?;
            write_len(out, s.len())?;
            out.extend(s.as_bytes());
        }
        "bytes" => {
            if !v.is_instance_of::<js_sys::Uint8Array>() {
                return Err(type_error("Expected Uint8Array"));
            }
            let bytes = js_sys::Uint8Array::new(v).to_vec();
            write_len(out, bytes.len())?;
            out.extend(bytes);
        }
        "timestamp" | "duration" => {
            let n = big_input(v)?;
            if n < 0 {
                return Err(type_error(if kind(&t) == "timestamp" {
                    "Timestamp outside Rust SystemTime range"
                } else {
                    "Negative duration is unsupported"
                }));
            }
            let seconds = u64::try_from(n / 1_000_000_000)
                .map_err(|_| type_error("Time outside supported range"))?;
            if kind(&t) == "timestamp" && seconds > i64::MAX as u64 {
                return Err(type_error("Timestamp outside supported range"));
            }
            out.extend(seconds.to_be_bytes());
            out.extend(((n % 1_000_000_000) as u32).to_be_bytes());
        }
        "optional" => {
            if v.is_null() || v.is_undefined() {
                out.push(0)
            } else {
                out.push(1);
                write(index(&t, "inner"), v, out, tx)?
            }
        }
        "sequence" => {
            let a = array(v)?;
            write_len(out, a.length() as usize)?;
            for v in a.iter() {
                write(index(&t, "inner"), &v, out, tx)?;
            }
        }
        "map" => {
            if !v.is_instance_of::<js_sys::Map>() {
                return Err(type_error("Expected Map"));
            }
            let m = v.unchecked_ref::<js_sys::Map>();
            write_len(out, m.size() as usize)?;
            for entry in js_sys::try_iter(v)?.ok_or_else(|| type_error("Invalid Map"))? {
                let e = js_sys::Array::from(&entry?);
                write(index(&t, "key"), &e.get(0), out, tx)?;
                write(index(&t, "value"), &e.get(1), out, tx)?;
            }
        }
        "record" => write_fields(&t["fields"], v, out, tx)?,
        "enum" => {
            let name = text(&get(v, "tag")?)?;
            let variants = t["variants"].as_array().unwrap();
            let i = variants
                .iter()
                .position(|x| x["name"] == name)
                .ok_or_else(|| type_error("Unknown exported enum variant"))?;
            out.extend(((i + 1) as i32).to_be_bytes());
            let f = &variants[i]["fields"];
            if t["flat"] == true {
                write_string(out, &get(v, "message")?.as_string().unwrap_or(name))?;
            } else if !f.as_array().unwrap().is_empty() {
                let value = get(v, "value")?;
                if f.as_array().unwrap().len() == 1 && f[0]["name"] == "0" {
                    write(index(&f[0], "ty"), &value, out, tx)?;
                } else {
                    write_fields(f, &value, out, tx)?;
                }
            }
        }
        "object" | "callback" => out.extend(object_input(&t, v, tx)?.to_be_bytes()),
        _ => unreachable!(),
    };
    Ok(())
}
fn write_len(out: &mut Vec<u8>, n: usize) -> Result<(), JsValue> {
    out.extend(
        i32::try_from(n)
            .map_err(|_| type_error("Value exceeds UniFFI buffer limit"))?
            .to_be_bytes(),
    );
    Ok(())
}
fn write_string(out: &mut Vec<u8>, s: &str) -> Result<(), JsValue> {
    write_len(out, s.len())?;
    out.extend(s.as_bytes());
    Ok(())
}
fn write_fields(
    fields: &Value,
    v: &JsValue,
    out: &mut Vec<u8>,
    tx: &mut Transaction,
) -> Result<(), JsValue> {
    if !v.is_object() {
        return Err(type_error("Expected an exported record"));
    }
    for f in fields.as_array().unwrap() {
        write(
            index(f, "ty"),
            &get(v, f["name"].as_str().unwrap())?,
            out,
            tx,
        )?;
    }
    Ok(())
}
struct Reader<'a> {
    bytes: &'a [u8],
}
impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], JsValue> {
        if n > self.bytes.len() {
            return Err(type_error("Truncated UniFFI wire value"));
        }
        let (a, b) = self.bytes.split_at(n);
        self.bytes = b;
        Ok(a)
    }
    fn len(&mut self) -> Result<usize, JsValue> {
        usize::try_from(i32::from_be_bytes(self.take(4)?.try_into().unwrap()))
            .map_err(|_| type_error("Invalid wire length"))
    }
}
fn read_fields(fields: &Value, r: &mut Reader<'_>) -> Result<JsValue, JsValue> {
    let tuple = fields
        .as_array()
        .unwrap()
        .first()
        .is_some_and(|f| f["name"] == "0");
    let value: JsValue = if tuple {
        js_sys::Array::new().into()
    } else {
        js_sys::Object::new().into()
    };
    for f in fields.as_array().unwrap() {
        let v = read(index(f, "ty"), r)?;
        put(&value, f["name"].as_str().unwrap(), &v)?;
    }
    Ok(value)
}
fn read(id: usize, r: &mut Reader<'_>) -> Result<JsValue, JsValue> {
    let t = ty(id);
    Ok(match kind(&t) {
        "custom" => read(index(&t, "inner"), r)?,
        "void" => JsValue::UNDEFINED,
        "bool" => JsValue::from_bool(r.take(1)?[0] != 0),
        k @ ("u8" | "i8" | "u16" | "i16" | "u32" | "i32" | "u64" | "i64") => {
            let bits = k[1..].parse::<usize>().unwrap();
            let bytes = r.take(bits / 8)?;
            let mut n = if k.starts_with('i') && bytes[0] & 128 != 0 {
                -1i128
            } else {
                0
            };
            for b in bytes {
                n = (n << 8) | (*b as i128);
            }
            if bits == 64 {
                big(n)
            } else {
                JsValue::from_f64(n as f64)
            }
        }
        "f32" => JsValue::from_f64(f32::from_be_bytes(r.take(4)?.try_into().unwrap()) as f64),
        "f64" => JsValue::from_f64(f64::from_be_bytes(r.take(8)?.try_into().unwrap())),
        "string" => {
            let n = r.len()?;
            JsValue::from_str(
                std::str::from_utf8(r.take(n)?).map_err(|_| type_error("Invalid UTF-8"))?,
            )
        }
        "bytes" => {
            let n = r.len()?;
            js_sys::Uint8Array::from(r.take(n)?).into()
        }
        "timestamp" | "duration" => {
            let seconds = if kind(&t) == "timestamp" {
                i64::from_be_bytes(r.take(8)?.try_into().unwrap()) as i128
            } else {
                u64::from_be_bytes(r.take(8)?.try_into().unwrap()) as i128
            };
            let nanos = u32::from_be_bytes(r.take(4)?.try_into().unwrap()) as i128;
            big(if seconds < 0 {
                seconds * 1_000_000_000 - nanos
            } else {
                seconds * 1_000_000_000 + nanos
            })
        }
        "optional" => {
            if r.take(1)?[0] == 0 {
                JsValue::NULL
            } else {
                read(index(&t, "inner"), r)?
            }
        }
        "sequence" => {
            let n = r.len()?;
            let a = js_sys::Array::new();
            for _ in 0..n {
                a.push(&read(index(&t, "inner"), r)?);
            }
            a.into()
        }
        "map" => {
            let n = r.len()?;
            let m = js_sys::Map::new();
            for _ in 0..n {
                let k = read(index(&t, "key"), r)?;
                let v = read(index(&t, "value"), r)?;
                m.set(&k, &v);
            }
            m.into()
        }
        "record" => read_fields(&t["fields"], r)?,
        "enum" => {
            let n = r.len()?;
            let v = t["variants"]
                .as_array()
                .unwrap()
                .get(
                    n.checked_sub(1)
                        .ok_or_else(|| type_error("Invalid enum wire tag"))?,
                )
                .ok_or_else(|| type_error("Unknown enum wire tag"))?;
            let o: JsValue = js_sys::Object::new().into();
            put(&o, "tag", &JsValue::from_str(v["name"].as_str().unwrap()))?;
            let fields = v["fields"].as_array().unwrap();
            if t["flat"] == true {
                let n = r.len()?;
                let _ = r.take(n)?;
            } else if !fields.is_empty() {
                let value = if fields.len() == 1 && fields[0]["name"] == "0" {
                    read(index(&fields[0], "ty"), r)?
                } else {
                    read_fields(&v["fields"], r)?
                };
                put(&o, "value", &value)?;
            }
            o
        }
        "object" | "callback" => {
            let h = u64::from_be_bytes(r.take(8)?.try_into().unwrap());
            object_output(&t, h)?
        }
        _ => unreachable!(),
    })
}
fn object_output(t: &Value, h: u64) -> Result<JsValue, JsValue> {
    if kind(t) == "callback" && JFFI_ABI_MODERN && h & 1 == 1 {
        let v = callback_value(h)?;
        callback_free(h);
        return Ok(v);
    }
    wrap_handle(index(t, "object"), h)
}
#[allow(non_camel_case_types)]
enum Raw {
    Void,
    i8(i8),
    u8(u8),
    i16(i16),
    u16(u16),
    i32(i32),
    u32(u32),
    i64(i64),
    u64(u64),
    f32(f32),
    f64(f64),
    Pointer(*const std::ffi::c_void),
    Buffer(Option<uniffi::RustBuffer>),
}
impl Drop for Raw {
    fn drop(&mut self) {
        if let Self::Buffer(b) = self {
            if let Some(b) = b.take() {
                b.destroy();
            }
        }
    }
}
macro_rules! raw_numbers {($($ty:ident=>$take:ident),*)=>{impl Raw {$(fn $take(&mut self)->$ty{match self{Self::$ty(v)=>*v,_=>unreachable!()}})*}}}
raw_numbers!(i8=>take_i8,u8=>take_u8,i16=>take_i16,u16=>take_u16,i32=>take_i32,u32=>take_u32,i64=>take_i64,u64=>take_u64,f32=>take_f32,f64=>take_f64);
impl Raw {
    fn take_rust_buffer(&mut self) -> uniffi::RustBuffer {
        match self {
            Self::Buffer(v) => v.take().unwrap(),
            _ => unreachable!(),
        }
    }
    fn take_pointer(&mut self) -> *const std::ffi::c_void {
        match self {
            Self::Pointer(v) => *v,
            _ => unreachable!(),
        }
    }
    fn take_void(&mut self) {}
    fn handle(&self) -> u64 {
        match self {
            Self::Pointer(v) => *v as u64,
            Self::u64(v) => *v,
            _ => unreachable!(),
        }
    }
}
fn lower(id: usize, v: &JsValue, tx: &mut Transaction) -> Result<Raw, JsValue> {
    let id = builtin(id);
    let t = ty(id);
    Ok(match kind(&t) {
        "void" => Raw::Void,
        "bool" => Raw::i8(v.as_bool().ok_or_else(|| type_error("Expected boolean"))? as i8),
        "u8" => Raw::u8(int(v, 8, false)? as u8),
        "i8" => Raw::i8(int(v, 8, true)? as i8),
        "u16" => Raw::u16(int(v, 16, false)? as u16),
        "i16" => Raw::i16(int(v, 16, true)? as i16),
        "u32" => Raw::u32(int(v, 32, false)? as u32),
        "i32" => Raw::i32(int(v, 32, true)? as i32),
        "u64" => Raw::u64(int(v, 64, false)? as u64),
        "i64" => Raw::i64(int(v, 64, true)? as i64),
        "f32" => Raw::f32(v.as_f64().ok_or_else(|| type_error("Expected number"))? as f32),
        "f64" => Raw::f64(v.as_f64().ok_or_else(|| type_error("Expected number"))?),
        "object" | "callback" => {
            let h = object_input(&t, v, tx)?;
            if JFFI_ABI_MODERN || t["box"] == true {
                Raw::u64(h)
            } else {
                Raw::Pointer(h as *const std::ffi::c_void)
            }
        }
        _ => {
            let mut bytes = Vec::new();
            if kind(&t) == "string" {
                bytes.extend(text(v)?.as_bytes())
            } else {
                write(id, v, &mut bytes, tx)?;
            }
            Raw::Buffer(Some(uniffi::RustBuffer::from_vec(bytes)))
        }
    })
}
fn lift(id: usize, mut raw: Raw) -> Result<JsValue, JsValue> {
    let t = ty(builtin(id));
    Ok(match kind(&t) {
        "void" => JsValue::UNDEFINED,
        "bool" => JsValue::from_bool(raw.take_i8() != 0),
        "i8" => JsValue::from_f64(raw.take_i8() as f64),
        "u8" => JsValue::from_f64(raw.take_u8() as f64),
        "i16" => JsValue::from_f64(raw.take_i16() as f64),
        "u16" => JsValue::from_f64(raw.take_u16() as f64),
        "i32" => JsValue::from_f64(raw.take_i32() as f64),
        "u32" => JsValue::from_f64(raw.take_u32() as f64),
        "i64" => big(raw.take_i64() as i128),
        "u64" => big(raw.take_u64() as i128),
        "f32" => JsValue::from_f64(raw.take_f32() as f64),
        "f64" => JsValue::from_f64(raw.take_f64()),
        "object" | "callback" => object_output(&t, raw.handle())?,
        _ => {
            let bytes = raw.take_rust_buffer().destroy_into_vec();
            if kind(&t) == "string" {
                JsValue::from_str(
                    std::str::from_utf8(&bytes).map_err(|_| type_error("Invalid UTF-8"))?,
                )
            } else {
                let mut r = Reader { bytes: &bytes };
                let value = read(id, &mut r)?;
                if !r.bytes.is_empty() {
                    return Err(type_error("Trailing UniFFI wire data"));
                }
                value
            }
        }
    })
}
fn check_status(mut status: Status, error: Option<usize>) -> Result<(), JsValue> {
    let code = status.code as i8;
    if code == 0 {
        return Ok(());
    }
    let bytes = unsafe { std::mem::ManuallyDrop::take(&mut status.error_buf) }.destroy_into_vec();
    if code == 1 {
        let err = js_sys::Error::new("Rust API returned an error");
        if let Some(id) = error {
            let details = read(id, &mut Reader { bytes: &bytes })?;
            put(err.as_ref(), "details", &details)?;
        }
        return Err(err.into());
    }
    Err(type_error(String::from_utf8_lossy(&bytes)))
}
fn finish(result: (Raw, Status), id: usize, error: Option<usize>) -> Result<JsValue, JsValue> {
    let (raw, status) = result;
    check_status(status, error)?;
    lift(id, raw)
}
fn invoke_callback(handle: u64, name: &str, args: &js_sys::Array) -> Result<JsValue, JsValue> {
    let v = callback_value(handle)?;
    get(&v, name)?
        .dyn_into::<js_sys::Function>()
        .map_err(|_| type_error("Invalid callback method"))?
        .apply(&v, args)
}
fn unexpected(e: JsValue) -> Status {
    let msg = if let Some(s) = e.as_string() {
        s
    } else if let Ok(s) = js_sys::Reflect::get(&e, &JsValue::from_str("message")) {
        s.as_string().unwrap_or_else(|| format!("{e:?}"))
    } else {
        format!("{e:?}")
    };
    Status::error(msg)
}
// Keep ownership staged until the native callback actually receives the value.
// A JS getter may reenter Rust and cancel an async call while it is being encoded.
struct CallbackOutcome {
    raw: Option<Raw>,
    status: Option<Status>,
    tx: Transaction,
}
impl CallbackOutcome {
    fn emit(mut self) -> Result<Raw, Status> {
        let status = self.status.take().unwrap();
        if matches!(&status.code, uniffi::RustCallStatusCode::Success) {
            self.tx.commit();
            Ok(self.raw.take().unwrap())
        } else {
            if matches!(&status.code, uniffi::RustCallStatusCode::Error) {
                self.tx.commit();
            }
            Err(status)
        }
    }
}
impl Drop for CallbackOutcome {
    fn drop(&mut self) {
        if let Some(mut s) = self.status.take() {
            unsafe { std::mem::ManuallyDrop::take(&mut s.error_buf) }.destroy();
        }
    }
}
fn callback_result(
    result: Result<JsValue, JsValue>,
    ret: usize,
    error: Option<usize>,
) -> CallbackOutcome {
    let mut tx = Transaction::new();
    let result = match result {
        Ok(v) => lower(ret, &v, &mut tx).map_err(unexpected),
        Err(e) => {
            let expected = (|| -> Option<Status> {
                let id = error?;
                let details = get(&e, "details").ok()?;
                if details.is_null() || details.is_undefined() {
                    return None;
                }
                let mut bytes = vec![];
                write(id, &details, &mut bytes, &mut tx).ok()?;
                let mut s = Status::default();
                s.code = uniffi::RustCallStatusCode::Error;
                s.error_buf = std::mem::ManuallyDrop::new(uniffi::RustBuffer::from_vec(bytes));
                Some(s)
            })();
            Err(expected.unwrap_or_else(|| unexpected(e)))
        }
    };
    let (raw, status) = match result {
        Ok(raw) => (Some(raw), Status::default()),
        Err(status) => (None, status),
    };
    CallbackOutcome {
        raw,
        status: Some(status),
        tx,
    }
}
// ABI-layout equivalents allow the same generated bridge to support both UniFFI versions.
#[repr(C)]
struct ForeignResult<T> {
    value: T,
    status: Status,
}
#[repr(C)]
struct ForeignTask {
    data: u64,
    free: extern "C" fn(u64),
}
struct Task {
    future: Option<Pin<Box<dyn Future<Output = ()>>>>,
    cancel: Option<Box<dyn FnOnce()>>,
    waker: Option<Waker>,
}
struct RunTask(u64);
impl Future for RunTask {
    type Output = ();
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        // Keep cancellation metadata registered while polling. JS can reenter
        // Rust and drop this future during the callback itself.
        let future = TASKS.with(|r| {
            let mut r = r.borrow_mut();
            r.get_mut(&self.0).and_then(|task| {
                task.waker = Some(cx.waker().clone());
                task.future.take()
            })
        });
        let Some(mut future) = future else {
            return Poll::Ready(());
        };
        let result = future.as_mut().poll(cx);
        TASKS.with(|r| {
            let mut r = r.borrow_mut();
            if result.is_ready() {
                r.remove(&self.0);
                Poll::Ready(())
            } else if let Some(task) = r.get_mut(&self.0) {
                task.future = Some(future);
                Poll::Pending
            } else {
                Poll::Ready(())
            }
        })
    }
}
extern "C" fn cancel_task(id: u64) {
    let task = TASKS.with(|r| r.borrow_mut().remove(&id));
    if let Some(mut task) = task {
        drop(task.future.take());
        if let Some(cancel) = task.cancel.take() {
            cancel();
        }
        if let Some(w) = task.waker.take() {
            w.wake();
        }
    }
}
fn spawn_task(
    f: impl Future<Output = ()> + 'static,
    cancel: impl FnOnce() + 'static,
) -> ForeignTask {
    let id = next();
    TASKS.with(|r| {
        r.borrow_mut().insert(
            id,
            Task {
                future: Some(Box::pin(f)),
                cancel: Some(Box::new(cancel)),
                waker: None,
            },
        )
    });
    wasm_bindgen_futures::spawn_local(RunTask(id));
    ForeignTask {
        data: id,
        free: cancel_task,
    }
}
extern "C" fn continuation(id: u64, ready: i8) {
    let w = FUTURES.with(|r| {
        let mut r = r.borrow_mut();
        r.get_mut(&id).map(|(done, w)| {
            *done = ready == 0;
            w.clone()
        })
    });
    if let Some(w) = w {
        w.wake();
    }
}
struct NativeFuture<T> {
    handle: u64,
    id: u64,
    poll: unsafe extern "C" fn(u64, extern "C" fn(u64, i8), u64),
    complete: unsafe extern "C" fn(u64, &mut Status) -> T,
    free: unsafe extern "C" fn(u64),
}
impl<T> Unpin for NativeFuture<T> {}
impl<T> Future for NativeFuture<T> {
    type Output = (T, Status);
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        let ready = FUTURES.with(|r| {
            let mut r = r.borrow_mut();
            let state = r
                .entry(this.id)
                .or_insert_with(|| (false, cx.waker().clone()));
            state.1 = cx.waker().clone();
            state.0
        });
        if !ready {
            unsafe { (this.poll)(this.handle, continuation, this.id) }
        }
        if FUTURES.with(|r| r.borrow().get(&this.id).unwrap().0) {
            let mut status = Status::default();
            let value = unsafe { (this.complete)(this.handle, &mut status) };
            Poll::Ready((value, status))
        } else {
            Poll::Pending
        }
    }
}
impl<T> Drop for NativeFuture<T> {
    fn drop(&mut self) {
        FUTURES.with(|r| r.borrow_mut().remove(&self.id));
        unsafe { (self.free)(self.handle) };
    }
}
