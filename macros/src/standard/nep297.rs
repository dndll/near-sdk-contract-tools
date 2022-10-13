use darling::{FromDeriveInput, FromVariant};
use proc_macro2::TokenStream;
use quote::quote;

use crate::rename::RenameStrategy;

#[derive(Debug, FromDeriveInput)]
#[darling(
    attributes(nep297),
    supports(struct_any, enum_any),
    and_then = "Self::check"
)]
pub struct Nep297Meta {
    pub standard: String,
    pub version: String,
    pub name: Option<String>,
    pub rename: Option<RenameStrategy>,
    pub rename_all: Option<RenameStrategy>,
    pub ident: syn::Ident,
    pub generics: syn::Generics,
    pub data: darling::ast::Data<EventVariantReceiver, ()>,

    // crates
    #[darling(rename = "crate", default = "crate::default_crate_name")]
    pub me: syn::Path,
}

macro_rules! disallow_field {
    ($self: ident, $field: ident, $e: ident, $shape: expr) => {
        if $self.$field.is_some() {
            $e.push(darling::Error::custom(format!(
                "The field `{}` is not allowed on {}s",
                stringify!($field),
                $shape,
            )));
        }
    };
}

impl Nep297Meta {
    pub fn check(self) -> darling::Result<Self> {
        let mut e = darling::Error::accumulator();

        match &self.data {
            darling::ast::Data::Enum(_) => {
                disallow_field!(self, name, e, "enum");
                disallow_field!(self, rename, e, "enum");
            }
            darling::ast::Data::Struct(_) => {
                disallow_field!(self, rename_all, e, "struct");
            }
        }

        e.finish_with(self)
    }
}

#[derive(Debug, FromVariant)]
#[darling(attributes(nep297))]
pub struct EventVariantReceiver {
    pub ident: syn::Ident,
    pub fields: darling::ast::Fields<()>,
    pub rename: Option<RenameStrategy>,
    pub name: Option<String>,
}

pub fn expand(meta: Nep297Meta) -> Result<TokenStream, darling::Error> {
    let Nep297Meta {
        standard,
        version,
        name,
        rename,
        rename_all,
        ident,
        generics,
        data,
        me,
    } = meta;

    let (imp, ty, wher) = generics.split_for_impl();

    // Variant attributes
    let event = match data {
        darling::ast::Data::Struct(_) => {
            let transformed_name = if let Some(name) = name {
                name
            } else if let Some(rename) = rename {
                rename.transform(ident.to_string())
            } else {
                ident.to_string()
            };

            quote! { #transformed_name }
        }
        darling::ast::Data::Enum(variants) => {
            let arms = variants.into_iter().map(|variant| {
                let i = &variant.ident;

                // This could be a function chain, but I found it to be unreadable
                let transformed_name = if let Some(name) = variant.name {
                    name
                } else if let Some(rename) = variant.rename.as_ref().or(rename_all.as_ref()) {
                    rename.transform(i.to_string())
                } else {
                    i.to_string()
                };

                match variant.fields.style {
                    darling::ast::Style::Tuple => {
                        quote! { Self :: #i ( .. ) => #transformed_name , }
                    }
                    darling::ast::Style::Struct => {
                        quote! { Self :: #i { .. } => #transformed_name , }
                    }
                    darling::ast::Style::Unit => {
                        quote! { Self :: #i  => #transformed_name , }
                    }
                }
            });

            quote! {
                match self {
                    #(#arms)*
                }
            }
        }
    };

    Ok(quote! {
        impl #imp #me::standard::nep297::Event<#ident #ty> for #ident #ty #wher {
            fn event_log<'__el>(&'__el self) -> #me::standard::nep297::EventLog<&'__el Self> {
                #me::standard::nep297::EventLog {
                    standard: #standard,
                    version: #version,
                    event: #event,
                    data: self,
                }
            }
        }
    })
}
