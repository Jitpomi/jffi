//! Foreign-language bridge: only UniFFI's stable exported ABI crosses this boundary.
use super::*;
use serde_json::Value;
#[derive(Default)]
pub(super) struct Contract {
    pub modern: bool,
    pub types: Vec<Value>,
    pub names: BTreeMap<String, usize>,
    pub calls: Vec<Call>,
    pub objects: Vec<Object>,
}
pub(super) struct Object {
    pub name: String,
    pub ty: usize,
    pub clone: String,
    pub free: String,
    pub init: Option<String>,
    pub callback: bool,
    pub methods: Vec<Call>,
}
#[derive(Clone)]
pub(super) struct Call {
    pub name: String,
    pub symbol: String,
    pub module: String,
    pub owner: Option<usize>,
    pub constructor: bool,
    pub asynchronous: bool,
    pub args: Vec<(String, usize)>,
    pub ret: usize,
    pub error: Option<usize>,
}
impl Contract {
    pub fn intern(&mut self, v: Value) -> usize {
        if let Some(i) = self.types.iter().position(|t| *t == v) {
            i
        } else {
            let i = self.types.len();
            self.types.push(v);
            i
        }
    }
    fn builtin(&self, mut id: usize) -> usize {
        while self.types[id]["kind"] == "custom" {
            id = self.types[id]["inner"].as_u64().unwrap() as usize;
        }
        id
    }
    fn ffi(&self, id: usize) -> &str {
        let t = &self.types[self.builtin(id)];
        match t["kind"].as_str().unwrap() {
            "bool" => "i8",
            "u8" => "u8",
            "i8" => "i8",
            "u16" => "u16",
            "i16" => "i16",
            "u32" => "u32",
            "i32" => "i32",
            "u64" => "u64",
            "i64" => "i64",
            "f32" => "f32",
            "f64" => "f64",
            "void" => "void",
            "object" | "callback" => {
                if self.modern || t["box"] == true {
                    "u64"
                } else {
                    "pointer"
                }
            }
            _ => "rust_buffer",
        }
    }
    fn ts(&self, id: usize) -> String {
        let t = &self.types[id];
        if let Some(n) = t["name"].as_str() {
            return n.into();
        }
        match t["kind"].as_str().unwrap() {
            "void" => "void".into(),
            "bool" => "boolean".into(),
            "u64" | "i64" => "bigint".into(),
            "string" => "string".into(),
            "bytes" => "JffiBytes".into(),
            "timestamp" => "JffiTimestamp".into(),
            "duration" => "JffiDuration".into(),
            "optional" => format!("{} | null", self.ts(t["inner"].as_u64().unwrap() as usize)),
            "sequence" => format!("Array<{}>", self.ts(t["inner"].as_u64().unwrap() as usize)),
            "map" => format!(
                "Map<{}, {}>",
                self.ts(t["key"].as_u64().unwrap() as usize),
                self.ts(t["value"].as_u64().unwrap() as usize)
            ),
            _ => "number".into(),
        }
    }
    fn fields_ts(&self, fields: &Value) -> String {
        let fields = fields.as_array().unwrap();
        if fields.first().is_some_and(|f| f["name"] == "0") {
            return format!(
                "[{}]",
                fields
                    .iter()
                    .map(|f| self.ts(f["ty"].as_u64().unwrap() as usize))
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
        format!(
            "{{ {} }}",
            fields
                .iter()
                .map(|f| format!(
                    "{}: {}",
                    f["name"].as_str().unwrap(),
                    self.ts(f["ty"].as_u64().unwrap() as usize)
                ))
                .collect::<Vec<_>>()
                .join("; ")
        )
    }
}
fn ffi_ty(kind: &str) -> TokenStream {
    match kind {
        "rust_buffer" => quote!(uniffi::RustBuffer),
        "pointer" => quote!(*const std::ffi::c_void),
        "void" => quote!(()),
        v => {
            let i = format_ident!("{v}");
            quote!(#i)
        }
    }
}
fn from_raw(kind: &str, expr: TokenStream) -> TokenStream {
    let variant = format_ident!(
        "{}",
        match kind {
            "rust_buffer" => "Buffer",
            "pointer" => "Pointer",
            "void" => "Void",
            v => v,
        }
    );
    if kind == "rust_buffer" {
        quote!(Raw::Buffer(Some(#expr)))
    } else if kind == "void" {
        quote!({#expr;Raw::Void})
    } else {
        quote!(Raw::#variant(#expr))
    }
}
fn take_raw(kind: &str, expr: TokenStream) -> TokenStream {
    let method = format_ident!("take_{kind}");
    quote!((#expr).#method())
}
fn opt(v: Option<usize>) -> TokenStream {
    match v {
        Some(v) => quote!(Some(#v)),
        None => quote!(None),
    }
}
fn raw_call(c: &Contract, call: &Call, id: usize) -> TokenStream {
    let helper = format_ident!("abi_call_{id}");
    let symbol = &call.symbol;
    let retkind = c.ffi(call.ret);
    let ret = ffi_ty(retkind);
    let raw_args: Vec<_> = call
        .args
        .iter()
        .enumerate()
        .map(|(i, _)| format_ident!("arg_{i}"))
        .collect();
    let ffi_args: Vec<_> = call.args.iter().map(|(_, t)| ffi_ty(c.ffi(*t))).collect();
    let lower: Vec<_> = call
        .args
        .iter()
        .enumerate()
        .map(|(i, (_, t))| {
            let n = i as u32;
            quote!(lower(#t,&args.get(#n),&mut tx)?)
        })
        .collect();
    let takes: Vec<_> = call
        .args
        .iter()
        .enumerate()
        .map(|(i, (_, t))| take_raw(c.ffi(*t), quote!(values[#i])))
        .collect();
    let receiver=call.owner.filter(|_|!call.constructor).map(|owner|{let kind=if c.modern{"u64"}else{"pointer"};let t=ffi_ty(kind);let value=if c.modern{quote!(receiver)}else{quote!(receiver as *const std::ffi::c_void)};(quote!(this:#t,),quote!(#value,),quote!{let receiver=clone_handle(#owner,receiver)?;tx.handles.push((#owner,receiver));})});
    let (receiver_decl, receiver_expr, receiver_clone) = receiver.unwrap_or_default();
    let asyncness = call.asynchronous.then(|| quote!(async));
    let call_body = if call.asynchronous {
        let prefix = format!("ffi_{}_rust_future", call.module);
        let poll = format!("{prefix}_poll_{retkind}");
        let complete = format!("{prefix}_complete_{retkind}");
        let free = format!("{prefix}_free_{retkind}");
        let wrap = from_raw(retkind, quote!(value));
        quote! {
            extern "C" {
                #[link_name=#symbol]fn native(#receiver_decl #(#raw_args:#ffi_args),*)->u64;
                #[link_name=#poll]fn poll(h:u64,cb:extern "C" fn(u64,i8),data:u64);
                #[link_name=#complete]fn complete(h:u64,status:&mut Status)->#ret;
                #[link_name=#free]fn free(h:u64);
            }
            let handle=unsafe{native(#receiver_expr #(#takes),*)};
            let (value,status)=NativeFuture{handle,id:next(),poll,complete,free}.await;
            Ok((#wrap,status))
        }
    } else {
        let wrap = from_raw(
            retkind,
            quote!(unsafe{native(#receiver_expr #(#takes,)* &mut status)}),
        );
        quote! {extern "C" {#[link_name=#symbol]fn native(#receiver_decl #(#raw_args:#ffi_args,)* status:&mut Status)->#ret;}let mut status=Status::default();let value=#wrap;Ok((value,status))}
    };
    quote! {#asyncness fn #helper(args:js_sys::Array,receiver:u64)->Result<(Raw,Status),JsValue>{initialize();let mut tx=Transaction::new();let mut values:Vec<Raw>=vec![#(#lower),*];#receiver_clone tx.commit();#call_body}}
}
fn api_call(c: &Contract, call: &Call, id: usize) -> (TokenStream, TokenStream) {
    let helper = format_ident!("abi_call_{id}");
    let function = format_ident!("exported_{id}");
    let name = &call.name;
    let asyncness = call.asynchronous.then(|| quote!(async));
    let await_ = call.asynchronous.then(|| quote!(.await));
    let mut externs = Vec::new();
    let mut args = Vec::new();
    let mut values = Vec::new();
    for (i, (name, t)) in call.args.iter().enumerate() {
        let arg = syn::parse_str::<syn::Ident>(name).unwrap_or_else(|_| format_ident!("r#{name}"));
        let ext = format_ident!("Arg{id}_{i}");
        let ts = c.ts(*t);
        externs.push(quote!(#[wasm_bindgen(typescript_type=#ts)]pub type #ext;));
        args.push(quote!(#arg:#ext));
        values.push(quote!(args.push(#arg.as_ref());));
    }
    let array = quote! {let args=js_sys::Array::new();#(#values)*};
    let ret = call.ret;
    let err = opt(call.error);
    let out = format_ident!("Return{id}");
    let ts = c.ts(ret);
    externs.push(quote!(#[wasm_bindgen(typescript_type=#ts)]pub type #out;));
    let receiver = if call.owner.is_some() && !call.constructor {
        quote!(&self,)
    } else {
        quote!()
    };
    let handle = if call.owner.is_some() && !call.constructor {
        quote!(self.handle)
    } else {
        quote!(0)
    };
    let body = if call.constructor {
        let owner = call.owner.unwrap();
        let class = format_ident!("AbiObject{owner}");
        quote! {let (raw,status)=#helper(args,0)#await_?;check_status(status,#err)?;Ok(#class::adopt(raw.handle()))}
    } else {
        quote!(Ok(finish(#helper(args,#handle)#await_?,#ret,#err)?.unchecked_into()))
    };
    let output = if call.constructor {
        let class = format_ident!("AbiObject{}", call.owner.unwrap());
        quote!(#class)
    } else {
        quote!(#out)
    };
    let attr = if call.constructor && name == "new" && !call.asynchronous {
        quote!(#[wasm_bindgen(constructor)])
    } else {
        quote!(#[wasm_bindgen(js_name=#name)])
    };
    (
        quote!(#(#externs)*),
        quote!(#attr pub #asyncness fn #function(#receiver #(#args),*)->Result<#output,JsValue>{#array #body}),
    )
}
fn callback_stub(
    c: &Contract,
    call: &Call,
    owner: usize,
    method: usize,
) -> (TokenStream, TokenStream, TokenStream) {
    let function = format_ident!("callback_{owner}_{method}");
    let field = format_ident!("method_{method}");
    let name = &call.name;
    let retid = call.ret;
    let err = opt(call.error);
    let retkind = c.ffi(retid);
    let ret = ffi_ty(retkind);
    let args: Vec<_> = call
        .args
        .iter()
        .enumerate()
        .map(|(i, _)| format_ident!("arg{i}"))
        .collect();
    let ffi_args: Vec<_> = call.args.iter().map(|(_, t)| ffi_ty(c.ffi(*t))).collect();
    let lift: Vec<_> = call
        .args
        .iter()
        .enumerate()
        .map(|(i, (_, t))| {
            let a = &args[i];
            let raw = from_raw(c.ffi(*t), quote!(#a));
            quote!(values.push(&lift(#t,#raw)?);)
        })
        .collect();
    let invoke = quote! {let result=(||->Result<JsValue,JsValue>{let values=js_sys::Array::new();#(#lift)* invoke_callback(handle,#name,&values)})();};
    let take = take_raw(retkind, quote!(raw));
    if call.asynchronous {
        let code = quote! {extern "C" fn #function(handle:u64,#(#args:#ffi_args,)* callback:extern "C" fn(u64,ForeignResult<#ret>),data:u64,out:&mut ForeignTask){
            #invoke
            // Observe the Promise immediately, including when native cancellation
            // happens before the spawned task receives its first poll.
            let result=result.map(|value|wasm_bindgen_futures::JsFuture::from(js_sys::Promise::resolve(&value)));
            let completed=std::rc::Rc::new(Cell::new(false));let completion=completed.clone();
            let cancelled=std::rc::Rc::new(Cell::new(false));let cancellation=cancelled.clone();
            *out=spawn_task(async move {
                let result=match result{Ok(value)=>value.await,Err(e)=>Err(e)};
                if completion.replace(true){return;}
                let outcome=callback_result(result,#retid,#err);
                let (value,status)=if cancellation.get(){drop(outcome);(<#ret as uniffi::FfiDefault>::ffi_default(),Status::cancelled())}else{match outcome.emit(){Ok(mut raw)=>(#take,Status::default()),Err(status)=>(<#ret as uniffi::FfiDefault>::ffi_default(),status)}};
                callback(data,ForeignResult{value,status});
            },move ||{cancelled.set(true);if !completed.replace(true){callback(data,ForeignResult{value:<#ret as uniffi::FfiDefault>::ffi_default(),status:Status::cancelled()});}});
        }};
        (
            code,
            quote!(#field:extern "C" fn(u64,#(#ffi_args,)*extern "C" fn(u64,ForeignResult<#ret>),u64,&mut ForeignTask)),
            quote!(#field:#function),
        )
    } else {
        let code = quote! {extern "C" fn #function(handle:u64,#(#args:#ffi_args,)* out:&mut #ret,status:&mut Status){#invoke match callback_result(result,#retid,#err).emit(){Ok(mut raw)=>*out=#take,Err(error)=>*status=error}}};
        (
            code,
            quote!(#field:extern "C" fn(u64,#(#ffi_args,)*&mut #ret,&mut Status)),
            quote!(#field:#function),
        )
    }
}
pub(super) fn render(module: &str, c: &Contract) -> Result<String> {
    let modern = c.modern;
    let schema = serde_json::to_string(&c.types)?;
    let runtime: TokenStream = include_str!("abi_runtime.rs")
        .parse()
        .map_err(|e| anyhow::anyhow!("Parsing ABI runtime: {e}"))?;
    let mut ts=String::from("export interface JffiRustError<T = unknown> extends Error { details: T }\nexport type JffiBytes = Uint8Array;\nexport type JffiTimestamp = bigint;\nexport type JffiDuration = bigint;\n");
    for (name, id) in &c.names {
        let t = &c.types[*id];
        match t["kind"].as_str().unwrap() {
            "record" => ts += &format!("export type {name} = {};\n", c.fields_ts(&t["fields"])),
            "custom" => ts += &format!("export type {name} = {};\n", c.ts(index_json(t, "inner"))),
            "enum" => {
                let variants = t["variants"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| {
                        let f = &v["fields"];
                        let fields = f.as_array().unwrap();
                        let payload = if fields.is_empty() {
                            String::new()
                        } else if fields.len() == 1 && fields[0]["name"] == "0" {
                            format!("; value: {}", c.ts(index_json(&fields[0], "ty")))
                        } else {
                            format!("; value: {}", c.fields_ts(f))
                        };
                        format!("{{ tag: \"{}\"{} }}", v["name"].as_str().unwrap(), payload)
                    })
                    .collect::<Vec<_>>()
                    .join(" | ");
                ts += &format!("export type {name} = {variants};\n");
            }
            "callback" => {
                let object = &c.objects[index_json(t, "object")];
                let members = object
                    .methods
                    .iter()
                    .map(|m| {
                        let ret = c.ts(m.ret);
                        format!(
                            "  {}({}): {};",
                            m.name,
                            m.args
                                .iter()
                                .map(|(n, t)| format!("{n}: {}", c.ts(*t)))
                                .collect::<Vec<_>>()
                                .join(", "),
                            if m.asynchronous {
                                format!("Promise<{ret}>")
                            } else {
                                ret
                            }
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                ts += &format!("export interface {name} {{\n{members}\n}}\n");
            }
            _ => {}
        }
    }
    let mut externs = Vec::new();
    let mut helpers = Vec::new();
    let mut exports = Vec::new();
    let mut methods = BTreeMap::<usize, Vec<TokenStream>>::new();
    let mut all = c.calls.clone();
    for o in &c.objects {
        if !c.types[o.ty]["box"].as_bool().unwrap_or(false) {
            all.extend(o.methods.clone());
        }
    }
    let mut exported_names = HashSet::new();
    for (id, call) in all.iter().enumerate() {
        if call.owner.is_none() && !exported_names.insert(call.name.clone()) {
            bail!("Conflicting standalone JavaScript export {}", call.name);
        }
        helpers.push(raw_call(c, call, id));
        let (ext, code) = api_call(c, call, id);
        externs.push(ext);
        if let Some(owner) = call.owner {
            methods.entry(owner).or_default().push(code)
        } else {
            exports.push(code);
        }
    }
    let mut classes = Vec::new();
    let mut clones = Vec::new();
    let mut frees = Vec::new();
    let mut wraps = Vec::new();
    let mut init = Vec::new();
    let mut callbacks = Vec::new();
    let mut callback_names = Vec::new();
    let handlekind = if c.modern { "u64" } else { "pointer" };
    let handletype = ffi_ty(handlekind);
    for (i, o) in c.objects.iter().enumerate() {
        let class = format_ident!("AbiObject{i}");
        let name = if o.callback {
            format!("__JffiRust{}", o.name)
        } else {
            o.name.clone()
        };
        let member = methods.remove(&i).unwrap_or_default();
        let names: Vec<_> = o.methods.iter().map(|m| m.name.as_str()).collect();
        callback_names.push(quote!(#i=>&[#(#names),*]));
        if let Some(symbol) = &o.init {
            let table = format_ident!("VTable{i}");
            let table_value = format_ident!("VTABLE{i}");
            let init_fn = format_ident!("init_callback_{i}");
            let mut fields = Vec::new();
            let mut values = Vec::new();
            for (n, m) in o.methods.iter().enumerate() {
                let (stub, field, value) = callback_stub(c, m, i, n);
                callbacks.push(stub);
                fields.push(field);
                values.push(value);
            }
            let free = quote!(free:extern "C" fn(u64));
            let free_value = quote!(free:callback_free);
            let extra = c.modern.then(|| quote!(clone:extern "C" fn(u64)->u64,));
            let extra_value = c.modern.then(|| quote!(clone:callback_clone,));
            let (fields, values) = if c.modern {
                (
                    quote!(#free,#extra #(#fields),*),
                    quote!(#free_value,#extra_value #(#values),*),
                )
            } else {
                (quote!(#(#fields,)*#free), quote!(#(#values,)*#free_value))
            };
            callbacks.push(quote!{#[repr(C)]struct #table{#fields}static #table_value:#table=#table{#values};extern "C"{#[link_name=#symbol]fn #init_fn(table:*const #table);}});
            init.push(quote!(unsafe{#init_fn(&#table_value)}));
        }
        if c.types[o.ty]["box"] == true {
            continue;
        }
        let clone_symbol = &o.clone;
        let free_symbol = &o.free;
        let h = if c.modern {
            quote!(handle)
        } else {
            quote!(handle as *const std::ffi::c_void)
        };
        let result = if c.modern {
            quote!(value)
        } else {
            quote!(value as u64)
        };
        clones.push(quote!(#i=>{extern "C"{#[link_name=#clone_symbol]fn native(h:#handletype,status:&mut Status)->#handletype;}let mut status=Status::default();let value=unsafe{native(#h,&mut status)};check_status(status,None)?;Ok(#result)}));
        frees.push(quote!(#i=>{extern "C"{#[link_name=#free_symbol]fn native(h:#handletype,status:&mut Status);}let mut status=Status::default();unsafe{native(#h,&mut status)};let _=check_status(status,None);}));
        wraps.push(quote!(#i=>Ok(JsValue::from(#class::adopt(handle)))));
        let hidden = o.callback.then(|| quote!(skip_typescript,));
        classes.push(quote!{#[wasm_bindgen(#hidden js_name=#name)]pub struct #class{handle:u64,id:u32}impl #class{fn adopt(handle:u64)->Self{let id=u32::try_from(next()).expect("Object reference limit");OBJECTS.with(|r|r.borrow_mut().insert(id,(#i,handle)));Self{handle,id}}}impl Drop for #class{fn drop(&mut self){OBJECTS.with(|r|r.borrow_mut().remove(&self.id));drop_handle(#i,self.handle);}}#[wasm_bindgen(js_class=#name)]impl #class{#[wasm_bindgen(getter,js_name=__jffiHandle,skip_typescript)]pub fn abi_handle(&self)->u32{self.id}#[wasm_bindgen(getter,js_name=__jffiType,skip_typescript)]pub fn abi_type(&self)->String{#name.into()}#(#member)*}});
    }
    let module = format_ident!("{module}");
    let tokens = quote! {
        #![cfg(target_arch="wasm32")]
        #![allow(dead_code,unused_variables,unused_mut,non_camel_case_types,non_snake_case,improper_ctypes,improper_ctypes_definitions)]
        const _:usize=std::mem::size_of::<#module::UniFfiTag>();
        const JFFI_ABI_SCHEMA:&str=#schema;const JFFI_ABI_MODERN:bool=#modern;
        #runtime
        fn initialize(){thread_local!{static INITIALIZED:Cell<bool>=const{Cell::new(false)};}INITIALIZED.with(|v|{if !v.replace(true){#(#init)*}});}
        fn clone_handle(object:usize,handle:u64)->Result<u64,JsValue>{match object{#(#clones,)* _=>Err(type_error("Cannot clone callback interface"))}}
        fn drop_handle(object:usize,handle:u64){match object{#(#frees,)* _=>{}}}
        fn wrap_handle(object:usize,handle:u64)->Result<JsValue,JsValue>{match object{#(#wraps,)* _=>Err(type_error("Cannot return a callback interface"))}}
        fn callback_methods(object:usize)->&'static[&'static str]{match object{#(#callback_names,)*_=>&[]}}
        #[wasm_bindgen]extern "C"{#(#externs)*}
        #(#callbacks)* #(#helpers)* #(#classes)* #(#exports)*
        #[wasm_bindgen(typescript_custom_section)]const JFFI_ABI_TYPES:&str=#ts;
    };
    Ok(format!(
        "{MARKER}\n{}",
        prettyplease::unparse(&syn::parse2(tokens)?)
    ))
}
fn index_json(t: &Value, k: &str) -> usize {
    t[k].as_u64().unwrap() as usize
}
