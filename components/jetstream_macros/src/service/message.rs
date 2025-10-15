use proc_macro2::TokenStream;
use quote::quote;
use syn::{Ident, Signature};

pub fn request_struct_name(method_name: &Ident) -> Ident {
    Ident::new(&format!("T{}", method_name), method_name.span())
}

pub fn return_struct_name(method_name: &Ident) -> Ident {
    Ident::new(&format!("R{}", method_name), method_name.span())
}

pub fn generate_msg_id(index: usize, method_name: &Ident) -> TokenStream {
    let upper_cased_method_name = method_name.to_string().to_uppercase();
    let tmsg_const_name = Ident::new(
        &format!("T{}", upper_cased_method_name),
        method_name.span(),
    );
    let rmsg_const_name = Ident::new(
        &format!("R{}", upper_cased_method_name),
        method_name.span(),
    );
    let offset = 2 * index as u8;

    quote! {
        pub const #tmsg_const_name: u8 = MESSAGE_ID_START + #offset;
        pub const #rmsg_const_name: u8 = MESSAGE_ID_START + #offset + 1;
    }
}

pub fn generate_input_struct(
    request_struct_ident: &Ident,
    method_sig: &Signature,
) -> TokenStream {
    let inputs = method_sig.inputs.iter().map(|arg| match arg {
        syn::FnArg::Typed(pat) => {
            let name = pat.pat.clone();
            let ty = pat.ty.clone();
            // if the pat is ::jetstream_rpc::context::Context, skip
            if let syn::Type::Path(type_path) = &*ty {
                if let Some(segment) = type_path.path.segments.last() {
                    if segment.ident == "Context" {
                        return quote! {};
                    }
                }
            }
            quote! {
                pub #name: #ty,
            }
        }
        syn::FnArg::Receiver(_) => quote! {},
    });

    quote! {
        #[allow(non_camel_case_types)]
        #[derive(Debug, JetStreamWireFormat)]
        pub struct #request_struct_ident {
            #(#inputs)*
        }
    }
}

pub fn generate_return_struct(
    return_struct_ident: &Ident,
    method_sig: &Signature,
) -> TokenStream {
    match &method_sig.output {
        syn::ReturnType::Type(_, ty) => {
            match &**ty {
                syn::Type::Path(type_path) => {
                    // Check if it's a Result type
                    if let Some(segment) = type_path.path.segments.last() {
                        if segment.ident == "Result" {
                            // Extract the success type from Result<T, E>
                            if let syn::PathArguments::AngleBracketed(args) =
                                &segment.arguments
                            {
                                if let Some(syn::GenericArgument::Type(
                                    success_type,
                                )) = args.args.first()
                                {
                                    return quote! {
                                        #[allow(non_camel_case_types)]
                                        #[derive(Debug, JetStreamWireFormat)]
                                        pub struct #return_struct_ident(pub #success_type);
                                    };
                                }
                            }
                        }
                    }
                    // If not a Result or couldn't extract type, use the whole type
                    quote! {
                        #[allow(non_camel_case_types)]
                        #[derive(Debug, JetStreamWireFormat)]
                        pub struct #return_struct_ident(pub #ty);
                    }
                }
                // Handle other return type variants if needed
                _ => {
                    quote! {
                        #[allow(non_camel_case_types)]
                        #[derive(Debug, JetStreamWireFormat)]
                        pub struct #return_struct_ident(pub #ty);
                    }
                }
            }
        }
        syn::ReturnType::Default => {
            quote! {
               #[allow(non_camel_case_types)]
               #[derive(Debug, JetStreamWireFormat)]
               pub struct #return_struct_ident;
            }
        }
    }
}
