extern crate proc_macro;
extern crate syn;

mod deref;
mod meta_collect;
mod meta_generate;

use meta_collect::{MetaCollectArgs, ToSynResult};
use proc_macro::TokenStream;
use syn::{parse_macro_input, ItemFn};

/// Automatically implement [`Deref`] trait for single filed structs.
///
/// # Example
///
/// ## Single-unnamed-field tuple struct
///
/// ```
/// use derive_deref::Deref;
///
/// #[derive(Deref)]
/// struct MyInteger(i32);
///
/// let foo = MyInteger(42_i32);
/// assert_eq!(42, *foo);
/// ```
///
/// ## Single-named-field struct
///
/// ```
/// use derive_deref::Deref;
///
/// #[derive(Deref)]
/// struct MyInteger {
///     value: i32,
/// };
///
/// let foo = MyInteger { value: 42_i32 };
/// assert_eq!(42, *foo);
///
/// ```
///
/// [`Deref`]: ::std::ops::Deref
#[proc_macro_derive(Deref)]
pub fn derive_deref(input: TokenStream) -> TokenStream {
    deref::derive_deref_impl(input, false)
}

/// Automatically implement [`DerefMut`] trait for single filed struct, requires a [`Deref`]
/// implementation.
///
/// # Example
///
/// ## Single-unnamed-field tuple struct
///
/// ```
/// use derive_deref::{Deref, DerefMut};
///
/// #[derive(Deref, DerefMut)]
/// struct MyInteger(i32);
///
/// let mut foo = MyInteger(0_i32);
/// *foo = 42;
/// assert_eq!(42, *foo);
/// ```
///
/// ## Single-named-field struct
///
/// ```
/// use derive_deref::{Deref, DerefMut};
///
/// #[derive(Deref, DerefMut)]
/// struct MyInteger {
///     value: i32,
/// };
///
/// let mut foo = MyInteger { value: 0_i32 };
/// *foo = 42;
/// assert_eq!(42, *foo);
/// ```
///
/// [`DerefMut`]: ::std::ops::DerefMut
/// [`Deref`]: ::std::ops::Deref
#[proc_macro_derive(DerefMut)]
pub fn derive_deref_mut(input: TokenStream) -> TokenStream {
    deref::derive_deref_impl(input, true)
}

/// This macro is the 2nd part of [`WgseCommand`] enumerate member auto-fill and trait [`WgseCommandExecute`]
/// auto-implemention.
///
/// # Notes
/// This macro is ONLY used for the '`meta_collect`' feature, otherwise a compilation error will occur.
///
/// # Example
/// ```
/// #[cfg(feature = "meta_collect")]
/// {
///     use meta_collect::wgse_command;
///
///     #[wgse_command(0x00, "Nope")]
///     pub fn execute_nope(/* arguments */) -> Result<()> {
///         /* implementions */
///     }
/// }
/// ```
///
/// [`WgseCommand`]: https://just.placeholder.url
/// [`WgseCommandExecute`]: https://just.placeholder.url
#[proc_macro_attribute]
pub fn wgse_command(args: TokenStream, input: TokenStream) -> TokenStream {
    let meta_args = parse_macro_input!(args as MetaCollectArgs);
    // clone and return original ast for rust-analyzer inspection in debug mode
    let func = input.clone();
    let mut ast = parse_macro_input!(func as ItemFn);

    match meta_collect::wgse_command_impl(meta_args, &mut ast).to_syn_result() {
        Ok(_) => (),
        Err(err) => return err.into_compile_error().into(),
    }

    #[cfg(debug_assertions)]
    {
        input
    }
    #[cfg(not(debug_assertions))]
    {
        TokenStream::new()
    }
}

/// This macro is the 1st part of [`WgseCommand`] enumerate member auto-fill and trait [`WgseCommandExecute`]
/// auto-implemention.
///
/// # Notes
/// This macro MUST be used for the `meta_init` feature and compile before use `wgse_command`
/// and `wgse_command_trait`.
///
/// # Example
/// ```
/// #[cfg(feature = "meta_init")]
/// use meta_gen::wgse_command_interface;
///
/// #[cfg(feature = "meta_init")]
/// pub trait WgseCommandExecute {
///     #[wgse_command_interface]
///     fn execute(&self, kernel: &mut VirtualMachine, args: &BinVec<Argument>) -> Result<()>;
/// }
///
/// ```
///
/// [`WgseCommand`]: https://just.placeholder.url
/// [`WgseCommandExecute`]: https://just.placeholder.url
#[proc_macro_attribute]
pub fn wgse_command_interface(arg: TokenStream, input: TokenStream) -> TokenStream {
    match meta_generate::wgse_command_interface_impl(arg, input).to_syn_result() {
        Ok(ast) => ast,
        Err(err) => err.into_compile_error().into(),
    }
}

/// This macro is the 3rd part of [`WgseCommand`] enumerate member auto-fill and trait [`WgseCommandExecute`]
/// auto-implemention.
///
/// # Notes
/// This macro is ONLY used without `meta_init` and `meta_collect` features.
///
/// # Example
/// ```
/// #[cfg(not(feature = "meta_collect"))]
/// {
///     use enum_dispatch::enum_dispatch;
///     #[cfg(not(feature = "meta_init"))]
///     use meta_gen::wgse_command_trait;
///
///     #[cfg(not(feature = "meta_init"))]
///     #[enum_dispatch]
///     pub trait WgseCommandExecute {
///         #[wgse_command_trait]
///         fn execute(&self, kernel: &mut VirtualMachine, args: &BinVec<Argument>) -> Result<()>;
///     }
/// }
/// ```
///
/// [`WgseCommand`]: https://just.placeholder.url
/// [`WgseCommandExecute`]: https://just.placeholder.url
#[proc_macro_attribute]
pub fn wgse_command_trait_impl(arg: TokenStream, input: TokenStream) -> TokenStream {
    match meta_generate::wgse_command_trait_impl(arg, input).to_syn_result() {
        Ok(ast) => ast,
        Err(err) => err.into_compile_error().into(),
    }
}
