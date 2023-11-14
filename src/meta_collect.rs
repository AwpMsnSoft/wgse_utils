use anyhow::Result as AnyResult;
use base64::{engine::general_purpose, Engine as _};
use convert_case::{Case, Casing};
use if_chain::if_chain;
use proc_macro::Span;
use quote::quote;
use serde_json::{json, Value};
use std::{
    env,
    error::Error,
    fmt::Display,
    fs::File,
    io::{BufReader, BufWriter, Read, Write},
    path::Path,
};
use syn::{
    parse,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    Error as SynError, FnArg, Ident, ItemFn, Lit, Pat, Result, Token, Visibility,
};

macro_rules! compile_err {
    ($msg: expr) => {{
        SynError::new(Span::call_site().into(), $msg)
    }};
}

#[derive(Debug, Clone)]
struct InvalidInterfaceError(String);

impl InvalidInterfaceError {
    fn new(msg: &str) -> Self {
        Self(msg.to_string())
    }
}

impl Display for InvalidInterfaceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Error for InvalidInterfaceError {}

pub trait ToSynResult<T> {
    fn to_syn_result(self) -> Result<T>;
}

impl<T> ToSynResult<T> for AnyResult<T> {
    fn to_syn_result(self) -> Result<T> {
        self.map_err(|err| compile_err!(format!("{:?}", err)))
    }
}

pub struct MetaCollectArgs {
    pub code: u8,
    pub name: String,
}

impl Parse for MetaCollectArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let err_msg = format!(
            "expect `#[wgse_command(<code: u8>, <name: str>)]` at {:?}",
            input.span()
        );
        let input_args = Punctuated::<Lit, Token![,]>::parse_terminated(input)?
            .into_iter()
            .collect::<Vec<_>>();

        if_chain! {
            if input_args.len() == 2;
            if let Lit::Int(ref code) = input_args[0];
            if let Lit::Str(ref name) = input_args[1];
            then {
                return Ok(Self {
                    code: code.base10_parse()?,
                    name: name.value()
                })
            }
        }

        Err(compile_err!(err_msg))
    }
}

pub fn wgse_command_impl(args: MetaCollectArgs, ast: &mut ItemFn) -> AnyResult<()> {
    let file_name = format!(
        "src/.autogen/wgse_command/{}.json",
        args.name.to_case(Case::Snake)
    );
    let project_dir = env::current_dir()?;
    let file_path = Path::new(&project_dir).join(file_name);

    let mut ast_clone = ast.clone();
    check_function_interface(&mut ast_clone)?;

    let receiver = quote! { &self }.into();
    ast.sig.inputs.insert(0, parse::<FnArg>(receiver)?);

    let autogen_payload = json!({
        "name": args.name,
        "code": args.code,
        "raw": quote!{ #ast }.to_string()
    });
    set_json_payload(&file_path, autogen_payload)?;

    Ok(())
}

fn preprocess_function_ast(ast: &mut ItemFn) -> AnyResult<()> {
    let receiver = quote! { &self }.into();

    ast.attrs = vec![];
    ast.vis = Visibility::Inherited;
    ast.sig.ident = Ident::new("_", Span::call_site().into());
    ast.sig.inputs.insert(0, parse::<FnArg>(receiver)?);
    ast.sig.inputs.iter_mut().for_each(|arg: &mut FnArg| {
        if_chain! {
            if let FnArg::Typed(arg) = arg;
            if let Pat::Ident(ref mut ident) = *arg.pat;
            then {
                ident.ident = Ident::new("_", Span::call_site().into());
            }
        }
    });
    Ok(())
}

fn check_function_interface(ast: &mut ItemFn) -> AnyResult<()> {
    preprocess_function_ast(ast)?;

    let project_dir = env::current_dir()?;
    let interface_path = Path::new(&project_dir).join("src/.autogen/interface.json");
    let interface_payload = get_json_payload(&interface_path)?;
    let interface_signature =
        interface_payload["raw"]
            .as_str()
            .ok_or(InvalidInterfaceError::new(
                "no interface signature found. run `cargo build --features meta_init` once before run `cargo build --features meta_collect`.",
            ))?;
    let func_signature = quote! { #(func.sig.clone()) }.to_string();

    if interface_signature == func_signature {
        Ok(())
    } else {
        Err(InvalidInterfaceError::new(&format!(
            "inconsistent interface signature. expect `{interface_signature}`, found `{func_signature}`."
        )))
    }?;
    Ok(())
}

fn get_json_payload(path: &Path) -> AnyResult<Value> {
    let mut content = String::new();
    BufReader::new(File::open(path)?).read_to_string(&mut content)?;

    let mut json_value = serde_json::from_str::<Value>(&content)?;
    json_value["raw"] = Value::String(String::from_utf8(
        general_purpose::STANDARD.decode(json_value["raw"].as_str().unwrap())?,
    )?);

    Ok(json_value)
}

fn set_json_payload(path: &Path, mut json_value: Value) -> AnyResult<()> {
    json_value["raw"] =
        Value::String(general_purpose::STANDARD.encode(json_value["raw"].as_str().unwrap()));
    BufWriter::new(File::create(path)?).write_all(json_value.to_string().as_bytes())?;
    Ok(())
}
