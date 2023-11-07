use proc_macro::{Span, TokenStream};
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Error, Index, Member, Result, Type};

macro_rules! compile_err {
    ($msg: expr) => {{
        Error::new(Span::call_site().into(), $msg)
    }};
}

pub fn derive_deref_impl(input: TokenStream, is_mut: bool) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);

    let ident = &ast.ident;
    let (member, ty) = match deref_member(&ast) {
        Ok(target) => target,
        Err(err) => return err.into_compile_error().into(),
    };

    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();

    if is_mut {
        quote! {
            impl #impl_generics ::std::ops::DerefMut for #ident #ty_generics #where_clause {
                fn deref_mut(&mut self) -> &mut Self::Target {
                    &mut self.#member
                }
            }
        }
        .into()
    } else {
        quote! {
            impl #impl_generics ::std::ops::Deref for #ident #ty_generics #where_clause {
                type Target = #ty;

                fn deref(&self) -> &Self::Target {
                    &self.#member
                }
            }
        }
        .into()
    }
}

fn deref_member(ast: &DeriveInput) -> Result<(Member, Type)> {
    if let Data::Struct(data) = &ast.data {
        if data.fields.len() > 1 {
            return Err(compile_err!(
                "cannot apply `derive_deref` on struct with multi fields."
            ));
        }

        let field = data
            .fields
            .iter()
            .next()
            .ok_or(compile_err!("cannot apply `derive_deref` on empty struct."))?;
        let member = field
            .ident
            .as_ref()
            .map(|named| Member::Named(named.clone()))
            .unwrap_or_else(|| Member::Unnamed(Index::from(0)));

        Ok((member, field.ty.clone()))
    } else {
        Err(compile_err!(
            "`derive_deref` can only be applied on struct."
        ))
    }
}
