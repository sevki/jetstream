use proc_macro2::TokenStream;
use quote::quote;
use syn::spanned::Spanned;

#[allow(dead_code)]
pub fn compile_error(span: impl Spanned, message: &str) -> TokenStream {
    let error = syn::Error::new(span.span(), message);
    error.to_compile_error()
}

pub fn unsupported_data_type() -> TokenStream {
    quote! {
        compile_error!("JetStreamWireFormat can only be derived for structs and enums");
    }
}
