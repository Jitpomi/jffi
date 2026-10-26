// Discover the API after rustc has expanded macros and selected conditional items.
use super::*;
use version::{Metadata, ObjectImpl, Type as UniType};
use wasmparser::{DataKind, ExternalKind, Operator, Parser, Payload, TypeRef};

fn constant(expr: wasmparser::ConstExpr<'_>) -> Result<u32> {
    match expr.get_operators_reader().read()? {
        Operator::I32Const { value } => Ok(value as u32),
        other => bail!("Unsupported metadata address expression: {other:?}"),
    }
}
fn extract(bytes: &[u8]) -> Result<Vec<Metadata>> {
    let mut globals = Vec::new();
    let mut exports = Vec::new();
    let mut segments = Vec::new();
    for payload in Parser::new(0).parse_all(bytes) {
        match payload? {
            Payload::ImportSection(section) => {
                for group in section {
                    // Rust's standalone wasm32 cdylibs do not import globals.
                    let group = group?;
                    for import in group.into_iter() {
                        if matches!(import?.1.ty, TypeRef::Global(_)) {
                            globals.push(None);
                        }
                    }
                }
            }
            Payload::GlobalSection(section) => {
                for global in section {
                    let global = global?;
                    globals.push(constant(global.init_expr).ok());
                }
            }
            Payload::ExportSection(section) => {
                for export in section {
                    let export = export?;
                    if export.kind == ExternalKind::Global
                        && export.name.starts_with("UNIFFI_META_")
                    {
                        exports.push((export.name.to_string(), export.index));
                    }
                }
            }
            Payload::DataSection(section) => {
                for segment in section {
                    let segment = segment?;
                    if let DataKind::Active {
                        memory_index: 0,
                        offset_expr,
                    } = segment.kind
                    {
                        segments.push((constant(offset_expr)?, segment.data));
                    }
                }
            }
            _ => {}
        }
    }
    exports.sort();
    let mut result = Vec::new();
    for (name, index) in exports {
        let address = globals
            .get(index as usize)
            .copied()
            .flatten()
            .with_context(|| format!("Missing metadata address for {name}"))?;
        let data = segments
            .iter()
            .find_map(|(start, data)| {
                address
                    .checked_sub(*start)
                    .and_then(|offset| data.get(offset as usize..))
            })
            .with_context(|| format!("Missing metadata bytes for {name}"))?;
        result.push(Metadata::read(data).with_context(|| {
            format!("Reading {name}; web metadata must match the project's UniFFI version")
        })?);
    }
    if result.is_empty() {
        bail!("Core WASM contains no UniFFI metadata; export APIs with UniFFI before building web bindings");
    }
    Ok(result)
}

pub(super) fn load(root: &Path, release: bool, library: &str) -> Result<Api> {
    // Select precisely the Core features enabled by the web adapter. Cargo handles
    // target predicates, .cargo configuration, build scripts and macro expansion.
    let cfg = BuildCfg::load(root, release)?;
    let core_manifest: toml::Value =
        toml::from_str(&fs::read_to_string(root.join("core/Cargo.toml"))?)?;
    let package = core_manifest["package"]["name"]
        .as_str()
        .context("Missing Core package name")?;
    let features: Vec<_> = cfg
        .facts
        .iter()
        .filter_map(|fact| {
            let meta = syn::parse_str::<syn::Meta>(fact).ok()?;
            let syn::Meta::NameValue(value) = meta else {
                return None;
            };
            if !value.path.is_ident("feature") {
                return None;
            }
            let syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Str(value),
                ..
            }) = value.value
            else {
                return None;
            };
            Some(value.value())
        })
        .collect();
    let mut command = std::process::Command::new("cargo");
    command.current_dir(root).args([
        "rustc",
        "--package",
        package,
        "--lib",
        "--target",
        "wasm32-unknown-unknown",
        "--no-default-features",
        "--message-format=json",
    ]);
    if !features.is_empty() {
        command.arg("--features").arg(features.join(","));
    }
    if release {
        command.arg("--release");
    }
    command.args(["--", "--crate-type=cdylib"]);
    crate::commands::build::configure_wasm_compiler(&mut command);
    let output = command
        .output()
        .context("Compiling Core UniFFI metadata for WASM")?;
    if !output.status.success() {
        let diagnostics = String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
            .filter_map(|v| v["message"]["rendered"].as_str().map(str::to_owned))
            .collect::<Vec<_>>()
            .join("\n");
        bail!(
            "Core WASM compilation failed:\n{}\n{}",
            String::from_utf8_lossy(&output.stderr),
            diagnostics
        );
    }
    let mut artifact = None;
    for line in String::from_utf8(output.stdout)?.lines() {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(line) {
            if value["reason"] == "compiler-artifact" && value["target"]["name"] == library {
                artifact = value["filenames"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|f| f.as_str())
                    .find(|f| f.ends_with(".wasm"))
                    .map(str::to_owned)
                    .or(artifact);
            }
        }
    }
    let artifact = artifact.context("Cargo did not produce a Core WASM library")?;
    let items = extract(&fs::read(artifact)?)?;
    use super::abi::{Call, Contract, Object};
    use serde_json::json;
    let mut contract = Contract {
        modern: MODERN,
        ..Contract::default()
    };
    let mut names = BTreeMap::new();
    for item in &items {
        let name = match item {
            Metadata::Record(v) => &v.name,
            Metadata::Enum(v) => &v.name,
            Metadata::Object(v) => &v.name,
            Metadata::CallbackInterface(v) => &v.name,
            Metadata::CustomType(v) => &v.name,
            _ => continue,
        };
        let key = (item.module_path().to_string(), name.clone());
        if names.contains_key(&key) {
            continue;
        }
        let count = items
            .iter()
            .filter(|other| match other {
                Metadata::Record(v) => v.name == *name,
                Metadata::Enum(v) => v.name == *name,
                Metadata::Object(v) => v.name == *name,
                Metadata::CallbackInterface(v) => v.name == *name,
                Metadata::CustomType(v) => v.name == *name,
                _ => false,
            })
            .count();
        let public = if count > 1 {
            format!(
                "{}{}",
                item.module_path()
                    .split("::")
                    .map(|s| camel(&format!("_{s}")))
                    .collect::<String>(),
                name
            )
        } else {
            name.clone()
        };
        let id = contract.types.len();
        contract.types.push(json!(null));
        if contract.names.insert(public.clone(), id).is_some() {
            bail!("Conflicting exported web name {public}");
        }
        names.insert(key, (id, public));
    }
    fn named(
        names: &BTreeMap<(String, String), (usize, String)>,
        module: &str,
        name: &str,
    ) -> Result<usize> {
        if let Some((id, _)) = names.get(&(module.into(), name.into())) {
            return Ok(*id);
        }
        let candidates: Vec<_> = names
            .iter()
            .filter(|((m, n), _)| n == name && m.split("::").next() == module.split("::").next())
            .collect();
        if candidates.len() == 1 {
            return Ok(candidates[0].1 .0);
        }
        bail!("Missing or ambiguous compiled UniFFI type {module}::{name}")
    }
    fn ty(
        c: &mut Contract,
        n: &BTreeMap<(String, String), (usize, String)>,
        t: &UniType,
    ) -> Result<usize> {
        let value = match t {
            UniType::UInt8 => json!({"kind":"u8"}),
            UniType::Int8 => json!({"kind":"i8"}),
            UniType::UInt16 => json!({"kind":"u16"}),
            UniType::Int16 => json!({"kind":"i16"}),
            UniType::UInt32 => json!({"kind":"u32"}),
            UniType::Int32 => json!({"kind":"i32"}),
            UniType::UInt64 => json!({"kind":"u64"}),
            UniType::Int64 => json!({"kind":"i64"}),
            UniType::Float32 => json!({"kind":"f32"}),
            UniType::Float64 => json!({"kind":"f64"}),
            UniType::Boolean => json!({"kind":"bool"}),
            UniType::String => json!({"kind":"string"}),
            UniType::Bytes => json!({"kind":"bytes"}),
            UniType::Timestamp => json!({"kind":"timestamp"}),
            UniType::Duration => json!({"kind":"duration"}),
            UniType::Record { module_path, name }
            | UniType::Enum { module_path, name }
            | UniType::Custom {
                module_path, name, ..
            }
            | UniType::Object {
                module_path, name, ..
            }
            | UniType::CallbackInterface { module_path, name } => {
                return named(n, module_path, name)
            }
            UniType::Optional { inner_type } => {
                json!({"kind":"optional","inner":ty(c,n,inner_type)?})
            }
            UniType::Sequence { inner_type } => {
                json!({"kind":"sequence","inner":ty(c,n,inner_type)?})
            }
            UniType::Map {
                key_type,
                value_type,
            } => json!({"kind":"map","key":ty(c,n,key_type)?,"value":ty(c,n,value_type)?}),
        };
        Ok(c.intern(value))
    }
    fn fields(
        c: &mut Contract,
        n: &BTreeMap<(String, String), (usize, String)>,
        f: &[version::FieldMetadata],
    ) -> Result<serde_json::Value> {
        Ok(json!(f
            .iter()
            .map(|f| Ok(json!({"name":camel(&f.name),"ty":ty(c,n,&f.ty)?})))
            .collect::<Result<Vec<_>>>()?))
    }
    let mut owners = BTreeMap::new();
    for item in &items {
        let (name, callback, boxed, clone, free, init) = match item {
            Metadata::Object(v) => (
                v.name.clone(),
                v.imp != ObjectImpl::Struct,
                false,
                v.clone_ffi_symbol_name(),
                v.free_ffi_symbol_name(),
                (v.imp == ObjectImpl::CallbackTrait)
                    .then(|| version::init_callback_vtable_fn_symbol_name(&v.module_path, &v.name)),
            ),
            Metadata::CallbackInterface(v) => (
                v.name.clone(),
                true,
                true,
                String::new(),
                String::new(),
                Some(version::init_callback_vtable_fn_symbol_name(
                    &v.module_path,
                    &v.name,
                )),
            ),
            _ => continue,
        };
        let id = named(&names, item.module_path(), &name)?;
        let index = contract.objects.len();
        owners.insert(id, index);
        let public = names[&(item.module_path().into(), name)].1.clone();
        contract.types[id] = json!({"kind":if callback{"callback"}else{"object"},"name":public,"object":index,"box":boxed,"foreign":init.is_some()});
        contract.objects.push(Object {
            name: public,
            ty: id,
            clone,
            free,
            init,
            callback,
            methods: Vec::new(),
        });
    }
    for item in &items {
        match item {
            Metadata::Record(v) => {
                let id = named(&names, &v.module_path, &v.name)?;
                let f = fields(&mut contract, &names, &v.fields)?;
                contract.types[id] = json!({"kind":"record","name":names[&(v.module_path.clone(),v.name.clone())].1,"fields":f});
            }
            Metadata::Enum(v) => {
                let id = named(&names, &v.module_path, &v.name)?;
                let variants = v
                    .variants
                    .iter()
                    .map(|v| {
                        Ok(json!({"name":v.name,"fields":fields(&mut contract,&names,&v.fields)?}))
                    })
                    .collect::<Result<Vec<_>>>()?;
                contract.types[id] = json!({"kind":"enum","name":names[&(v.module_path.clone(),v.name.clone())].1,"variants":variants,"flat":matches!(v.shape,version::EnumShape::Error{flat:true})});
            }
            Metadata::CustomType(v) => {
                let id = named(&names, &v.module_path, &v.name)?;
                let inner = ty(&mut contract, &names, &v.builtin)?;
                contract.types[id] = json!({"kind":"custom","name":names[&(v.module_path.clone(),v.name.clone())].1,"inner":inner});
            }
            _ => {}
        }
    }
    let void = contract.intern(json!({"kind":"void"}));
    let mut traits = BTreeMap::<usize, Vec<(u32, Call)>>::new();
    for item in &items {
        let (module, name, symbol, inputs, ret, error, asynchronous, owner, constructor, index) =
            match item {
                Metadata::Func(v) if v.module_path.split("::").next() == Some(library) => (
                    &v.module_path,
                    &v.name,
                    v.ffi_symbol_name(),
                    &v.inputs,
                    v.return_type.as_ref(),
                    v.throws.as_ref(),
                    v.is_async,
                    None,
                    false,
                    None,
                ),
                Metadata::Constructor(v) => (
                    &v.module_path,
                    &v.name,
                    v.ffi_symbol_name(),
                    &v.inputs,
                    None,
                    v.throws.as_ref(),
                    v.is_async,
                    Some(named(&names, &v.module_path, &v.self_name)?),
                    true,
                    None,
                ),
                Metadata::Method(v) => (
                    &v.module_path,
                    &v.name,
                    v.ffi_symbol_name(),
                    &v.inputs,
                    v.return_type.as_ref(),
                    v.throws.as_ref(),
                    v.is_async,
                    Some(named(&names, &v.module_path, &v.self_name)?),
                    false,
                    None,
                ),
                Metadata::TraitMethod(v) => (
                    &v.module_path,
                    &v.name,
                    v.ffi_symbol_name(),
                    &v.inputs,
                    v.return_type.as_ref(),
                    v.throws.as_ref(),
                    v.is_async,
                    Some(named(&names, &v.module_path, &v.trait_name)?),
                    false,
                    Some(v.index),
                ),
                _ => continue,
            };
        let args = inputs
            .iter()
            .map(|p| Ok((camel(&p.name), ty(&mut contract, &names, &p.ty)?)))
            .collect::<Result<Vec<_>>>()?;
        let ret = if constructor {
            owner.unwrap()
        } else {
            ret.map(|t| ty(&mut contract, &names, t))
                .transpose()?
                .unwrap_or(void)
        };
        let call = Call {
            name: camel(name),
            symbol,
            module: module.split("::").next().unwrap().into(),
            owner: owner.map(|id| owners[&id]),
            constructor,
            asynchronous,
            args,
            ret,
            error: error.map(|t| ty(&mut contract, &names, t)).transpose()?,
        };
        if let Some(index) = index {
            traits
                .entry(call.owner.unwrap())
                .or_default()
                .push((index, call));
        } else {
            contract.calls.push(call);
        }
    }
    for (owner, mut methods) in traits {
        methods.sort_by_key(|(i, _)| *i);
        contract.objects[owner].methods = methods.into_iter().map(|(_, m)| m).collect();
    }
    // The compiler-selected ABI is the only source of native names and shapes.
    // No source indexing, Rust visibility, reexports or dependency imports are required.
    Ok(Api {
        abi: Some(contract),
        ..Api::default()
    })
}
