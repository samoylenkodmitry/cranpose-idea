use proc_macro::TokenStream;
use quote::quote;
use syn::{FnArg, ItemFn, parse_macro_input};

/// Installs a hot-call boundary inside a composable in a private development build.
#[proc_macro_attribute]
pub fn hot(_args: TokenStream, input: TokenStream) -> TokenStream {
    let mut function = parse_macro_input!(input as ItemFn);
    let mut arguments = Vec::new();
    let mut patterns = Vec::new();
    for (index, input) in function.sig.inputs.iter_mut().enumerate() {
        let FnArg::Typed(argument) = input else {
            return syn::Error::new_spanned(input, "hot composables must be free functions")
                .to_compile_error()
                .into();
        };
        patterns.push(argument.pat.clone());
        let name = quote::format_ident!("__cpdev_arg_{index}");
        *argument.pat = syn::parse_quote!(#name);
        arguments.push(name);
    }
    let block = function.block;
    function.block = syn::parse_quote!({
        crate::__cranpose_dev::observe_patch();
        crate::__cranpose_dev::call((#(#arguments,)*), |(#(#patterns,)*)| #block)
    });
    quote!(#function).into()
}
