use darling::{util::Override, FromDeriveInput};
use proc_macro2::TokenStream;
use quote::quote;
use syn::Expr;

const DEFAULT_STORAGE_KEY: &str = r#"(b"~$141" as &[u8])"#;

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(nep141), supports(struct_named))]
pub struct Nep141Meta {
    pub storage_key: Option<Expr>,
    pub hook: Option<Override<syn::TypePath>>,

    pub generics: syn::Generics,
    pub ident: syn::Ident,
}

pub fn expand(meta: Nep141Meta) -> Result<TokenStream, darling::Error> {
    let Nep141Meta {
        storage_key,
        hook,

        generics,
        ident,
    } = meta;

    let (imp, ty, wher) = generics.split_for_impl();

    let storage_key =
        storage_key.unwrap_or_else(|| syn::parse_str::<Expr>(DEFAULT_STORAGE_KEY).unwrap());

    let impl_empty_hook = if hook.is_none() {
        Some(quote! {
            impl #imp near_contract_tools::standard::nep141::Nep141Hook for #ident #ty #wher {}
        })
    } else {
        None
    };

    let hook = hook
        .map(|hook| match hook {
            Override::Inherit => quote! { Self }, // Nep141Hook implemented on Self
            Override::Explicit(t) => quote! { #t }, // Nep141Hook implemented on t
        })
        .unwrap_or_else(|| quote! { Self }); // implicit Nep141 implementation (no hooks)

    let hook = quote! { <#hook as near_contract_tools::standard::nep141::Nep141Hook> };

    fn wrap(f: Option<syn::ExprPath>) -> Option<TokenStream> {
        f.map(|f| {
            quote! {
                #f();
            }
        })
    }

    fn wrap_args(f: Option<syn::ExprPath>) -> Option<TokenStream> {
        f.map(|f| {
            quote! {
                #f(self, &sender_id, &receiver_id, amount, memo.as_deref());
            }
        })
    }

    fn wrap_call_args(f: Option<syn::ExprPath>) -> Option<TokenStream> {
        f.map(|f| {
            quote! {
                #f(self, &sender_id, &receiver_id, amount, memo.as_deref(), &msg);
            }
        })
    }

    Ok(quote! {
        #impl_empty_hook

        impl #imp near_contract_tools::standard::nep141::Nep141Controller for #ident #ty #wher {
            fn root(&self) -> near_contract_tools::slot::Slot<()> {
                near_contract_tools::slot::Slot::root(#storage_key)
            }
        }

        #[near_sdk::near_bindgen]
        impl #imp near_contract_tools::standard::nep141::Nep141 for #ident #ty #wher {
            #[payable]
            fn ft_transfer(
                &mut self,
                receiver_id: near_sdk::AccountId,
                amount: near_sdk::json_types::U128,
                memo: Option<String>,
            ) {
                use near_contract_tools::{
                    event::Event,
                    standard::nep141::{Nep141Controller, Nep141Event},
                };

                near_sdk::assert_one_yocto();
                let sender_id = near_sdk::env::predecessor_account_id();
                let amount: u128 = amount.into();

                #hook::before_transfer();

                // #before_transfer_plain_args
                // #before_transfer_args

                // #before_transfer_plain
                // #before_transfer

                Nep141Controller::transfer(self, &sender_id, &receiver_id, amount, memo.as_deref());

                // #after_transfer_plain_args
                // #after_transfer_args

                // #after_transfer_plain
                // #after_transfer
            }

            #[payable]
            fn ft_transfer_call(
                &mut self,
                receiver_id: near_sdk::AccountId,
                amount: near_sdk::json_types::U128,
                memo: Option<String>,
                msg: String,
            ) -> near_sdk::Promise {
                near_sdk::assert_one_yocto();
                let sender_id = near_sdk::env::predecessor_account_id();
                let amount: u128 = amount.into();

                // #before_transfer_call_args
                // #before_transfer_args

                // #before_transfer_call
                // #before_transfer

                let r = near_contract_tools::standard::nep141::Nep141Controller::transfer_call(
                    self,
                    sender_id,
                    receiver_id,
                    amount,
                    memo.as_deref(),
                    msg,
                    near_sdk::env::prepaid_gas(),
                );

                // #after_transfer_call_args
                // #after_transfer_args

                // #after_transfer_call
                // #after_transfer

                r
            }

            fn ft_total_supply(&self) -> near_sdk::json_types::U128 {
                near_contract_tools::standard::nep141::Nep141Controller::total_supply(self).into()
            }

            fn ft_balance_of(&self, account_id: near_sdk::AccountId) -> near_sdk::json_types::U128 {
                near_contract_tools::standard::nep141::Nep141Controller::balance_of(self, &account_id).into()
            }
        }

        #[near_sdk::near_bindgen]
        impl #imp near_contract_tools::standard::nep141::Nep141Resolver for #ident #ty #wher {
            #[private]
            fn ft_resolve_transfer(
                &mut self,
                sender_id: near_sdk::AccountId,
                receiver_id: near_sdk::AccountId,
                amount: near_sdk::json_types::U128,
            ) -> near_sdk::json_types::U128 {
                near_contract_tools::standard::nep141::Nep141Controller::resolve_transfer(self, sender_id, receiver_id, amount.into()).into()
            }
        }
    })
}
