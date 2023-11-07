use anyhow::Result as AnyResult;
use base64::{engine::general_purpose, Engine as _};
use convert_case::{Case, Casing};
use if_chain::if_chain;
use proc_macro::{Span, TokenStream};
use quote::quote;
use serde_json::{json, Value};
use std::{
    env,
    fs::File,
    io::{BufReader, BufWriter, Read, Write},
    path::Path,
};
use syn::{parse, parse_str, FnArg, Ident, ItemEnum, ItemFn, Pat, TraitItemFn, Visibility};
use walkdir::WalkDir;

pub fn wgse_command_interface_impl(_: TokenStream, input: TokenStream) -> AnyResult<TokenStream> {
    let project_name = env::current_dir()?;
    let file_path = project_name.join("src/.autogen/interface.json");

    let mut func = parse::<TraitItemFn>(input.clone())?;
    func.attrs = vec![];
    func.sig.ident = Ident::new("_", Span::call_site().into());
    func.sig.inputs.iter_mut().for_each(|arg| {
        if_chain! {
            if let FnArg::Typed(arg) = arg;
            if let Pat::Ident(ref mut ident) = *arg.pat;
            then {
                ident.ident = Ident::new("_", Span::call_site().into());
            }
        }
    });

    set_json_payload(&file_path, json! {{"raw": quote! {#func}.to_string()}})?;
    Ok(input)
}

pub fn wgse_command_trait_impl(arg: TokenStream, input: TokenStream) -> AnyResult<TokenStream> {
    let project_dir = env::current_dir()?;
    let dest_dir = Path::new(&project_dir).join("src/.autogen/wgse_commands");

    let trait_name = parse::<Ident>(arg)?;
    let target_enum = parse::<ItemEnum>(input)?;

    let (mut commands_ast, tag_list) = parse_command_files(&dest_dir, &trait_name)?;

    let tags = tag_list.into_iter().map(|tag| quote! { #tag, });

    // NOTE: the default command MUST be `.Nope`
    let variant_ast = Into::<TokenStream>::into(quote! {
        #[enum_dispatch(#trait_name)]
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub enum #target_enum
        {
            #(#tags)*
        }

        impl ::std::default::Default for #target_enum {
            fn default() -> Self {
                #target_enum::Nope(Nope)
            }
        }
    });

    commands_ast.extend(vec![variant_ast]);

    Ok(commands_ast)
}

fn parse_command_files(
    dest_dir: &Path,
    trait_name: &Ident,
) -> AnyResult<(TokenStream, Vec<Ident>)> {
    let mut tag_list = vec![];
    let mut ast = TokenStream::new();

    for entry in WalkDir::new(dest_dir)
        .into_iter()
        .filter_map(|path| path.ok())
        .filter(|path| path.file_type().is_file())
    {
        let json_value = get_json_payload(entry.path())?;

        let name = json_value["name"]
            .as_str()
            .unwrap()
            .to_case(Case::UpperCamel);
        let code = json_value["code"].as_u64().unwrap();
        let mut func = parse_str::<ItemFn>(json_value["raw"].as_str().unwrap())?;

        func.sig.ident = Ident::new("execute", Span::call_site().into());
        func.vis = Visibility::Inherited;

        // command name as variant member
        tag_list.push(parse_str::<Ident>(&name)?);

        // command code constant
        let const_name = parse_str::<Ident>(&name.to_case(Case::UpperSnake))?;
        let const_ast = quote! {
            const #const_name: u8 = #code;
        };

        // tag struct as command
        let tag_name = parse_str::<Ident>(&name)?;
        let tag_name_ast = quote! {
            #[derive(Debug, Default, Clone, Eq, PartialEq)]
            pub struct #tag_name;
        };

        // implement trait for tag struct
        let impl_trait_ast = quote! {
            impl #trait_name for #tag_name {
                #func
            }
        };

        // append tokenstream
        let command_ast: Vec<TokenStream> =
            vec![const_ast.into(), tag_name_ast.into(), impl_trait_ast.into()];
        ast.extend(command_ast);
    }
    Ok((ast, tag_list))
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
