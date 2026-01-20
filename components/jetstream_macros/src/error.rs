//! Error macro for creating rich Jetstream errors with source location.

use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{Expr, LitStr, Token};

mod kw {
    syn::custom_keyword!(code);
    syn::custom_keyword!(severity);
    syn::custom_keyword!(help);
    syn::custom_keyword!(url);
    syn::custom_keyword!(message);

    // Severity levels
    syn::custom_keyword!(Error);
    syn::custom_keyword!(Warning);
    syn::custom_keyword!(Advice);
}

#[derive(Clone, Copy)]
enum Severity {
    Error,
    Warning,
    Advice,
}

enum PartialError {
    Code(LitStr),
    Severity(Severity),
    Help(LitStr),
    Url(LitStr),
    Message(LitStr, Vec<Expr>),
}

pub struct ErrorMacro {
    code: Option<LitStr>,
    severity: Option<Severity>,
    help: Option<LitStr>,
    url: Option<LitStr>,
    message: LitStr,
    format_args: Vec<Expr>,
}

impl syn::parse::Parse for ErrorMacro {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut partials = vec![];

        while !input.is_empty() {
            let lookahead = input.lookahead1();

            if lookahead.peek(kw::code) {
                input.parse::<kw::code>()?;
                input.parse::<Token![:]>()?;
                partials.push(PartialError::Code(input.parse()?));
            } else if lookahead.peek(kw::severity) {
                input.parse::<kw::severity>()?;
                input.parse::<Token![:]>()?;
                let lookahead = input.lookahead1();
                if lookahead.peek(kw::Error) {
                    input.parse::<kw::Error>()?;
                    partials.push(PartialError::Severity(Severity::Error));
                } else if lookahead.peek(kw::Warning) {
                    input.parse::<kw::Warning>()?;
                    partials.push(PartialError::Severity(Severity::Warning));
                } else if lookahead.peek(kw::Advice) {
                    input.parse::<kw::Advice>()?;
                    partials.push(PartialError::Severity(Severity::Advice));
                } else {
                    return Err(lookahead.error());
                }
            } else if lookahead.peek(kw::help) {
                input.parse::<kw::help>()?;
                input.parse::<Token![:]>()?;
                partials.push(PartialError::Help(input.parse()?));
            } else if lookahead.peek(kw::url) {
                input.parse::<kw::url>()?;
                input.parse::<Token![:]>()?;
                partials.push(PartialError::Url(input.parse()?));
            } else if lookahead.peek(kw::message) {
                input.parse::<kw::message>()?;
                input.parse::<Token![:]>()?;
                let msg: LitStr = input.parse()?;

                // Parse format args if any
                let mut format_args = Vec::new();
                while input.peek(Token![,])
                    && !input.peek2(kw::code)
                    && !input.peek2(kw::severity)
                    && !input.peek2(kw::help)
                    && !input.peek2(kw::url)
                    && !input.peek2(kw::message)
                {
                    input.parse::<Token![,]>()?;
                    if !input.is_empty()
                        && !input.peek(kw::code)
                        && !input.peek(kw::severity)
                        && !input.peek(kw::help)
                        && !input.peek(kw::url)
                        && !input.peek(kw::message)
                    {
                        format_args.push(input.parse()?);
                    } else {
                        break;
                    }
                }
                partials.push(PartialError::Message(msg, format_args));
            } else {
                return Err(lookahead.error());
            }

            // Trailing comma is optional
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }

        let code = partials.iter().find_map(|p| match p {
            PartialError::Code(s) => Some(s.clone()),
            _ => None,
        });

        let severity = partials.iter().find_map(|p| match p {
            PartialError::Severity(s) => Some(*s),
            _ => None,
        });

        let help = partials.iter().find_map(|p| match p {
            PartialError::Help(s) => Some(s.clone()),
            _ => None,
        });

        let url = partials.iter().find_map(|p| match p {
            PartialError::Url(s) => Some(s.clone()),
            _ => None,
        });

        let (message, format_args) = partials
            .iter()
            .find_map(|p| match p {
                PartialError::Message(m, args) => {
                    Some((m.clone(), args.clone()))
                }
                _ => None,
            })
            .expect("message: must be provided");

        Ok(ErrorMacro {
            code,
            severity,
            help,
            url,
            message,
            format_args,
        })
    }
}

impl From<proc_macro::TokenStream> for ErrorMacro {
    fn from(value: proc_macro::TokenStream) -> Self {
        syn::parse(value).unwrap()
    }
}

impl From<ErrorMacro> for proc_macro::TokenStream {
    fn from(val: ErrorMacro) -> Self {
        val.to_token_stream().into()
    }
}

impl ToTokens for ErrorMacro {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let message = &self.message;
        let format_args = &self.format_args;

        // Build the message with format args
        let message_expr = if format_args.is_empty() {
            quote! { #message.to_string() }
        } else {
            quote! { format!(#message, #(#format_args),*) }
        };

        // Start building the diagnostic
        let mut builder = quote! {
            let mut __diag = ::miette::MietteDiagnostic::new(#message_expr);
        };

        // Add code if present
        if let Some(code) = &self.code {
            builder = quote! {
                #builder
                __diag = __diag.with_code(#code);
            };
        }

        // Add severity if present
        if let Some(severity) = &self.severity {
            let severity_token = match severity {
                Severity::Error => quote! { ::miette::Severity::Error },
                Severity::Warning => quote! { ::miette::Severity::Warning },
                Severity::Advice => quote! { ::miette::Severity::Advice },
            };
            builder = quote! {
                #builder
                __diag = __diag.with_severity(#severity_token);
            };
        }

        // Add help if present
        if let Some(help) = &self.help {
            builder = quote! {
                #builder
                __diag = __diag.with_help(#help);
            };
        }

        // Add url if present
        if let Some(url) = &self.url {
            builder = quote! {
                #builder
                __diag = __diag.with_url(#url);
            };
        }

        // Generate the final error with source location
        tokens.extend(quote! {
            {
                #builder
                ::jetstream_error::Error::from(__diag)
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;

    #[test]
    fn test_parse_simple_message() {
        let input: TokenStream = quote! { message: "simple error" };
        let parsed: ErrorMacro = syn::parse2(input).unwrap();
        assert_eq!(parsed.message.value(), "simple error");
    }

    #[test]
    fn test_parse_with_code() {
        let input: TokenStream = quote! {
            code: "jetstream::test",
            message: "error message"
        };
        let parsed: ErrorMacro = syn::parse2(input).unwrap();
        assert_eq!(parsed.code.unwrap().value(), "jetstream::test");
        assert_eq!(parsed.message.value(), "error message");
    }

    #[test]
    fn test_parse_with_format_args() {
        let input: TokenStream = quote! {
            message: "error: {}", some_var
        };
        let parsed: ErrorMacro = syn::parse2(input).unwrap();
        assert_eq!(parsed.message.value(), "error: {}");
        assert_eq!(parsed.format_args.len(), 1);
    }

    #[test]
    fn test_parse_with_severity_error() {
        let input: TokenStream = quote! {
            severity: Error,
            message: "an error"
        };
        let parsed: ErrorMacro = syn::parse2(input).unwrap();
        assert!(matches!(parsed.severity, Some(Severity::Error)));
    }

    #[test]
    fn test_parse_with_severity_warning() {
        let input: TokenStream = quote! {
            severity: Warning,
            message: "a warning"
        };
        let parsed: ErrorMacro = syn::parse2(input).unwrap();
        assert!(matches!(parsed.severity, Some(Severity::Warning)));
    }

    #[test]
    fn test_parse_with_severity_advice() {
        let input: TokenStream = quote! {
            severity: Advice,
            message: "some advice"
        };
        let parsed: ErrorMacro = syn::parse2(input).unwrap();
        assert!(matches!(parsed.severity, Some(Severity::Advice)));
    }

    #[test]
    fn test_parse_full() {
        let input: TokenStream = quote! {
            code: "jetstream::rpc::timeout",
            severity: Error,
            help: "increase timeout",
            url: "https://docs.rs",
            message: "request timed out after {}ms", timeout
        };
        let parsed: ErrorMacro = syn::parse2(input).unwrap();
        assert_eq!(parsed.code.unwrap().value(), "jetstream::rpc::timeout");
        assert!(matches!(parsed.severity, Some(Severity::Error)));
        assert_eq!(parsed.help.unwrap().value(), "increase timeout");
        assert_eq!(parsed.url.unwrap().value(), "https://docs.rs");
        assert_eq!(parsed.message.value(), "request timed out after {}ms");
        assert_eq!(parsed.format_args.len(), 1);
    }

    #[test]
    fn test_to_tokens() {
        let input: TokenStream = quote! {
            code: "jetstream::test",
            message: "test error"
        };
        let parsed: ErrorMacro = syn::parse2(input).unwrap();
        let output = parsed.to_token_stream();
        assert!(!output.is_empty());
    }
}
