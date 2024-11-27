use crate::{
    core::{compile::{ast::*, build::BuildOutput}, util::*, CoreError}, data::SEAHORSE_UTIL, match1
};
use proc_macro2::{Ident, Literal as PM2Literal, TokenStream};
use quote::{format_ident, quote, ToTokens};
use regex::Regex;
#[cfg(not(target_arch = "wasm32"))]
use rustfmt_wrapper::{config::*, rustfmt_config, Error as RustfmtError};
use std::{cell::RefCell, collections::BTreeSet, rc::Rc};

use super::compile::builtin::prelude::MethodType;

pub struct GenerateOutput {
    pub tree: Tree<String>,
    pub features: BTreeSet<Feature>,
}

/// A set of features that need to be turned on in order to compile an artifact.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Feature {
    Pyth,
}

impl Feature {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Pyth => "pyth-sdk-solana",
        }
    }
}

/// Convenience function for turning strings into Idents
fn ident<S: ToString>(name: &S) -> Ident {
    format_ident!("{}", name.to_string())
}

/// Newtype for a `::`-separated path.
struct StaticPath<'a>(&'a Vec<String>);
impl ToTokens for StaticPath<'_> {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let Self(path) = self;
        let path = path.iter().map(|part| ident(part));

        tokens.extend(quote! { #(#path)::* });
    }
}

impl ToTokens for Artifact {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let Self {
            constants,
            uses,
            type_defs,
            functions,
            ..
        } = self;

        tokens.extend(quote! {
            #![allow(unused_imports)]
            #![allow(unused_variables)]
            #![allow(unused_mut)]

            // Default imports
            use crate::{id, seahorse_util::*};
            use std::{rc::Rc, cell::RefCell};
            use anchor_lang::{prelude::*, solana_program};
            // TODO might not need these, contexts are defined in lib.rs now
            use anchor_spl::token::{self, Token, Mint, TokenAccount};

            #(#uses)*
            #(#constants)*
            #(#type_defs)*
            #(#functions)*
        });
    }
}

impl ToTokens for Use {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let Use { rooted, tree } = self;

        if !tree.is_dead() {
            tokens.extend(if *rooted {
                quote! { use crate::#tree; }
            } else {
                quote! { use #tree; }
            });
        }
    }
}

impl ToTokens for Tree<Option<String>> {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        tokens.extend(match self {
            Self::Leaf(None) => quote! {},
            Self::Leaf(Some(alias)) => {
                let alias = ident(alias);

                quote! { as #alias }
            }
            Self::Node(node) => {
                let node = node
                    .iter()
                    .filter_map(|(name, tree)| {
                        if tree.is_dead() {
                            return None;
                        }

                        let name = ident(name);

                        match tree {
                            Tree::Leaf(None) => Some(quote! { #name }),
                            Tree::Leaf(Some(alias)) => {
                                let alias = ident(alias);

                                Some(quote! { #name as #alias })
                            }
                            tree @ Tree::Node(..) => Some(quote! { #name::#tree }),
                        }
                    })
                    .collect::<Vec<_>>();

                if node.len() == 1 {
                    quote! { #(#node)* }
                } else {
                    quote! { {#(#node),*} }
                }
            }
        })
    }
}

impl ToTokens for Constant {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self { name, value } = self;
        let name = ident(name);

        tokens.extend(quote! {
            seahorse_const! { #name, #value }
        });
    }
}

impl ToTokens for TypeDef {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        tokens.extend(match self {
            Self::Struct(type_def) => quote! { #type_def },
            Self::Account(type_def) => quote! { #type_def },
            Self::Enum(type_def) => quote! { #type_def },
        });
    }
}

fn loaded_field(expr: TokenStream, ty: &TyExpr) -> TokenStream {
    let ty_expr = StoredTyExpr(ty);

    match ty {
        // Vec<T> special case, works a bit like Array
        TyExpr::Generic { name, params, .. } if name == &["Vec"] => {
            let inner = loaded_field(quote! { element }, &params[0]);

            quote! {
                Mutable::new(#expr.into_iter().map(|element| #inner).collect())
            }
        }
        TyExpr::Generic {
            is_loadable,
            mutability,
            ..
        } => {
            let inner = match is_loadable {
                false => quote! { #expr },
                true => quote! { #ty_expr::load(#expr) },
            };

            match mutability {
                Mutability::Immutable => inner,
                Mutability::Mutable => quote! { Mutable::new(#inner) },
            }
        }
        TyExpr::Array { element, .. } => {
            let inner = loaded_field(quote! { element }, &**element);

            quote! { Mutable::new(#expr.map(|element| #inner)) }
        }
        TyExpr::Tuple(tuple) => {
            let inner = tuple
                .iter()
                .enumerate()
                .map(|(index, ty)| loaded_field(quote! { tuple.#index }, ty));

            quote! {
                {
                    let tuple = #expr;
                    (#(#inner),*)
                }
            }
        }
        _ => todo!(),
    }
}

fn stored_field(expr: TokenStream, ty: &TyExpr) -> TokenStream {
    let ty_expr = StoredTyExpr(ty);

    match ty {
        TyExpr::Generic { name, params, .. } if name == &["Vec"] => {
            let inner = stored_field(quote! { element }, &params[0]);

            quote! {
                #expr.borrow().clone().into_iter().map(|element| #inner).collect()
            }
        }
        TyExpr::Generic {
            is_loadable,
            mutability,
            ..
        } => {
            let inner = match mutability {
                Mutability::Immutable => expr,
                Mutability::Mutable => quote! { #expr.borrow().clone() },
            };

            match is_loadable {
                false => inner,
                true => quote! { #ty_expr::store(#inner) },
            }
        }
        TyExpr::Array { element, .. } => {
            let inner = stored_field(quote! { element }, &**element);

            quote! {
                #expr.borrow().clone().map(|element| #inner)
            }
        }
        TyExpr::Tuple(tuple) => {
            let inner = tuple
                .iter()
                .enumerate()
                .map(|(index, ty)| stored_field(quote! { tuple.#index }, ty));

            quote! {
                {
                    let tuple = #expr;
                    (#(#inner),*)
                }
            }
        }
        _ => todo!(),
    }
}

impl ToTokens for Struct {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let Self {
            name,
            fields,
            methods,
            constructor,
            is_event,
            is_dataclass,
        } = self;
        let stored_name = ident(name);
        let name = ident(&format!("Loaded{}", name));

        let mut instance_methods = vec![];
        let mut static_methods = vec![];

        if let Some(func) = constructor {
            // This might look esoteric but it's actually pretty much exactly how Python uses
            // constructors under the hood - a __new__ method calls the user-defined __init__ to
            // perform the heavy lifting of the constructor.
            let ext_params = func.params.iter().map(|(name, ty)| {
                let name = ident(name);
                let ty = LoadedTyExpr(ty);

                quote! { #name: #ty }
            });

            let ext_param_names = func.params.iter().map(|(name, _)| {
                let name = ident(name);

                quote! { #name }
            });

            let func = InstanceMethod(func);

            instance_methods.push(quote! { #func });

            static_methods.push(quote! {
                pub fn __new__(#(#ext_params),*) -> Mutable<Self> {
                    let obj = Mutable::new(#name::default());
                    obj.__init__(#(#ext_param_names),*);
                    return obj;
                }
            });
        } else if *is_dataclass {
            let ctor_params = fields.iter().map(|(name, ty_expr, _)| {
                let name = ident(name);
                let ty_expr = LoadedTyExpr(ty_expr);

                quote! { #name: #ty_expr }
            });

            let ctor_param_names = fields.iter().map(|(name, _, _)| {
                let name = ident(name);

                quote! { #name }
            });

            static_methods.push(quote! {
                pub fn __new__(#(#ctor_params), *) -> Mutable<Self> {
                    let obj = #name { #(#ctor_param_names),* };
                    return Mutable::new(obj);
                }
            });
        }

        for (method_type, func) in methods.iter() {
            match method_type {
                MethodType::Instance => {
                    let method = InstanceMethod(func);

                    instance_methods.push(quote! { #method });
                }
                MethodType::Static => {
                    static_methods.push(quote! { #func });
                }
            }
        }

        // Split up the instance methods and static methods: each instance method of a class will
        // belong to an `impl Mutable<Class>` block, and the static methods will belong to an
        // `impl Class` block.

        let event_emit_fn = if *is_event {
            let fs = fields.iter().map(|(name, ty, original_ty)| {
                let name = ident(name);

                let needs_clone = !original_ty.is_copy();
                let field = if needs_clone {
                    quote! { e.#name.clone() }
                } else {
                    quote! { e.#name }
                };

                let field = stored_field(quote! { #field }, ty);

                quote! { #name: #field }
            });

            Some(quote! {
                fn __emit__(&self) {
                    let e = self.borrow();
                    emit!(#stored_name { #(#fs),* })
                }
            })
        } else {
            None
        };

        let instance_impl = if instance_methods.len() > 0 || event_emit_fn.is_some() {
            Some(quote! { impl Mutable<#name> {
                #(#instance_methods)*

                #event_emit_fn
            }})
        } else {
            None
        };

        let static_impl = if static_methods.len() > 0 {
            Some(quote! { impl #name { #(#static_methods)* } })
        } else {
            None
        };

        let stored_macros = if *is_event {
            quote! { #[event] }
        } else {
            quote! { #[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)] }
        };

        let macros = quote! { #[derive(Clone, Debug, Default)] };

        let stored_fields = fields.iter().map(|(name, ty, _)| {
            let name = ident(name);
            let ty = StoredTyExpr(ty);

            quote! { pub #name: #ty }
        });

        let load_fields = fields.iter().map(|(name, ty, _)| {
            let name = ident(name);
            let field = loaded_field(quote! { stored.#name }, ty);

            quote! { #name: #field }
        });

        let store_fields = fields.iter().map(|(name, ty, orig_ty)| {
            let name = ident(name);
            let field = if orig_ty.is_copy() {
                quote! { loaded.#name }
            } else {
                quote! { loaded.#name.clone() }
            };
            let field = stored_field(field, ty);

            quote! { #name: #field }
        });

        let fields = fields.iter().map(|(name, ty, _)| {
            let name = ident(name);
            let ty = LoadedTyExpr(ty);

            quote! { pub #name: #ty }
        });

        tokens.extend(quote! {
            #stored_macros
            pub struct #stored_name { #(#stored_fields),* }

            #macros
            pub struct #name { #(#fields),* }

            #instance_impl

            #static_impl

            impl Loadable for #stored_name {
                type Loaded = #name;

                fn load(stored: Self) -> Self::Loaded {
                    Self::Loaded { #(#load_fields),* }
                }

                fn store(loaded: Self::Loaded) -> Self {
                    Self { #(#store_fields),* }
                }
            }
        });
    }
}

impl ToTokens for Account {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let Self {
            name,
            fields,
            methods,
        } = self;

        let account_name = ident(name);
        let loaded_name = ident(&format!("Loaded{}", name));

        let account_fields = fields.iter().map(|(name, ty, _)| {
            let name = ident(name);
            let ty = StoredTyExpr(ty);

            quote! { pub #name: #ty }
        });

        let loaded_fields = fields.iter().map(|(name, ty_expr, _)| {
            let name = ident(name);
            let ty_expr = LoadedTyExpr(ty_expr);

            quote! { pub #name: #ty_expr }
        });

        let loads = fields.iter().map(|(name, ty, orig_ty)| {
            let name = ident(name);
            let field = if orig_ty.is_copy() {
                quote! { account.#name }
            } else {
                quote! { account.#name.clone() }
            };
            let field = loaded_field(field, ty);

            quote! { let #name = #field; }
        });

        let field_names = fields.iter().map(|(name, ..)| {
            let name = ident(name);

            quote! { #name }
        });

        let store_fields = fields.iter().map(|(name, ty, orig_ty)| {
            let name = ident(name);
            let field = if orig_ty.is_copy() {
                quote! { loaded.#name }
            } else {
                quote! { loaded.#name.clone() }
            };
            let field = stored_field(field, ty);

            quote! {
                let #name = #field;
                loaded.__account__.#name = #name;
            }
        });

        let mut instance_methods = vec![];
        let mut static_methods = vec![];

        for (method_type, func) in methods.iter() {
            match method_type {
                MethodType::Instance => {
                    let method = InstanceMethod(func);

                    instance_methods.push(quote! { #method });
                }
                MethodType::Static => {
                    static_methods.push(quote! { #func });
                }
            }
        }

        // Like regular structs, split up the instance methods and static methods: each instance
        // method of an account will belong to an `impl Mutable<LoadedAccount<'_, '_>>` block, and
        // the static methods will belong to the original `impl Account` block.

        let instance_impl = if instance_methods.len() > 0 {
            Some(quote! { impl Mutable<#loaded_name<'_, '_>> { #(#instance_methods)* } })
        } else {
            None
        };

        tokens.extend(quote! {
            #[account]
            #[derive(Debug)]
            pub struct #account_name { #(#account_fields),* }

            impl<'info, 'entrypoint> #account_name {
                pub fn load(account: &'entrypoint mut Box<Account<'info, Self>>, programs_map: &'entrypoint ProgramsMap<'info>) -> Mutable<#loaded_name<'info, 'entrypoint>> {
                    #(#loads)*
                    Mutable::new(#loaded_name {
                        __account__: account,
                        __programs__: programs_map,
                        #(#field_names),*
                    })
                }

                pub fn store(loaded: Mutable<#loaded_name>) {
                    let mut loaded = loaded.borrow_mut();
                    #(#store_fields)*
                }

                #(#static_methods)*
            }

            #[derive(Debug)]
            pub struct #loaded_name<'info, 'entrypoint> {
                pub __account__: &'entrypoint mut Box<Account<'info, #account_name>>,
                pub __programs__: &'entrypoint ProgramsMap<'info>,
                #(#loaded_fields),*
            }

            #instance_impl
        });
    }
}

impl ToTokens for Enum {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let Self { name, variants } = self;
        let name = ident(name);
        let variants_tokens = variants.iter().map(|(name, _)| {
            let name = ident(name);

            quote! { #name }
        });

        let (first_variant, _) = &variants[0];
        let first_variant = ident(first_variant);

        tokens.extend(quote! {
            #[derive(Clone, Debug, PartialEq, AnchorSerialize, AnchorDeserialize, Copy)]
            pub enum #name {
                #(#variants_tokens),*
            }

            impl Default for #name {
                fn default() -> Self {
                    #name::#first_variant
                }
            }
        });
    }
}

/// Newtype to display the "loaded" (used at runtime) type of a type expression.
///
/// Note that there isn't a `ToTokens` implementation for `TyExpr` itself - you
/// need to choose either `LoadedTyExpr` or `StoredTyExpr` for context.
pub struct LoadedTyExpr<'a>(pub &'a TyExpr);
impl<'a> ToTokens for LoadedTyExpr<'a> {
    // Mutability is relevant in this context and defined types need to be their Loaded- counterpart
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(match self.0 {
            TyExpr::Generic {
                name,
                params,
                mutability,
                is_loadable,
            } => {
                let path = StaticPath(name);
                let params = match params.len() {
                    0 => quote! {},
                    _ => {
                        let params = params.iter().map(|param| LoadedTyExpr(param));

                        quote! { <#(#params),*> }
                    }
                };

                let inner = if *is_loadable {
                    quote! { Loaded!(#path #params) }
                } else {
                    quote! { #path #params }
                };

                match mutability {
                    Mutability::Immutable => inner,
                    Mutability::Mutable => quote! { Mutable<#inner> },
                }
            }
            TyExpr::Array { element, size } => {
                let element = LoadedTyExpr(element.as_ref());
                let size = LoadedTyExpr(size.as_ref());

                quote! { Mutable<[#element; #size]> }
            }
            TyExpr::Tuple(tuple) => {
                let tuple = tuple.iter().map(|element| LoadedTyExpr(element));

                quote! { (#(#tuple),*) }
            }
            TyExpr::Account(path) => {
                let mut path = path.clone();
                *path.last_mut().unwrap() = format!("Loaded{}", path.last().unwrap());
                let path = StaticPath(&path);

                quote! { Mutable<#path<'info, '_>> }
            }
            TyExpr::Const(size) => {
                let size = PM2Literal::usize_unsuffixed(*size);

                quote! { #size }
            }
            TyExpr::InfoLifetime => quote! { 'info },
            TyExpr::AnonLifetime => quote! { '_ },
        })
    }
}

/// Newtype to display the "stored" (used in on-chain interface) type of a type expression.
struct StoredTyExpr<'a>(&'a TyExpr);
impl<'a> ToTokens for StoredTyExpr<'a> {
    // Mutability is irrelevant in this context
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(match self.0 {
            TyExpr::Generic { name, params, .. } => {
                let path = StaticPath(name);
                let params = match params.len() {
                    0 => quote! {},
                    _ => {
                        let params = params.iter().map(|param| StoredTyExpr(param));

                        quote! { <#(#params),*> }
                    }
                };

                quote! { #path #params }
            }
            TyExpr::Array { element, size } => {
                let element = StoredTyExpr(element.as_ref());
                let size = StoredTyExpr(size.as_ref());

                quote! { [#element; #size] }
            }
            TyExpr::Tuple(tuple) => {
                let tuple = tuple.iter().map(|element| LoadedTyExpr(element));

                quote! { (#(#tuple),*) }
            }
            TyExpr::Account(path) => {
                let path = StaticPath(&path);

                quote! { #path<'info, '_> }
            }
            TyExpr::Const(size) => {
                let size = PM2Literal::usize_unsuffixed(*size);

                quote! { #size }
            }
            TyExpr::InfoLifetime => quote! { 'info },
            TyExpr::AnonLifetime => quote! { '_ },
        })
    }
}

impl ToTokens for Function {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let Self {
            ix_context,
            name,
            info_lifetime,
            params,
            returns,
            body,
        } = self;

        let name = if ix_context.is_some() {
            ident(&format!("{}_handler", name))
        } else {
            ident(name)
        };

        let info_lifetime = if *info_lifetime {
            Some(quote! { <'info> })
        } else {
            None
        };

        let params = params.iter().map(|(name, ty)| {
            let name = ident(name);
            let ty = LoadedTyExpr(ty);

            quote! { mut #name: #ty }
        });

        let returns = LoadedTyExpr(returns);

        tokens.extend(quote! {
            pub fn #name #info_lifetime(#(#params),*) -> #returns #body
        });
    }
}

/// Newtype for an instance method of a mutable type.
struct InstanceMethod<'a>(&'a Function);

impl<'a> ToTokens for InstanceMethod<'a> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Function {
            ix_context,
            name,
            info_lifetime,
            params,
            returns,
            body,
        } = self.0;

        let name = if ix_context.is_some() {
            ident(&format!("{}_handler", name))
        } else {
            ident(name)
        };

        let info_lifetime = if *info_lifetime {
            Some(quote! { <'info> })
        } else {
            None
        };

        let params = [quote! { &self }]
            .into_iter()
            .chain(params.iter().map(|(name, ty)| {
                let name = ident(name);
                let ty = LoadedTyExpr(ty);

                quote! { mut #name: #ty }
            }));

        let returns = LoadedTyExpr(returns);

        tokens.extend(quote! {
            pub fn #name #info_lifetime(#(#params),*) -> #returns #body
        });
    }
}

impl ToTokens for InstructionContext {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self {
            name,
            params,
            accounts,
            inferred_accounts,
        } = self;

        let name = ident(name);

        let params = match params.len() {
            0 => quote! {},
            _ => {
                let params = params.iter().map(|(name, ty_expr)| {
                    let name = ident(name);
                    let ty_expr = StoredTyExpr(ty_expr);

                    quote! { #name: #ty_expr }
                });

                quote! {
                    #[instruction(#(#params),*)]
                }
            }
        };

        let accounts =
            accounts
                .iter()
                .map(
                    |(
                        name,
                        ContextAccount {
                            annotation,
                            account_ty,
                            // ty,
                            ..
                        },
                    )| {
                        let name = ident(name);
                        let annotation = annotation
                            .as_ref()
                            .map(|annotation| AccountAnnotationWithTyExpr(annotation, account_ty));

                        quote! {
                            #annotation
                            pub #name: #account_ty
                        }
                    },
                )
                .chain(inferred_accounts.iter().map(
                    |(name, ContextAccount { account_ty, .. })| {
                        let name = ident(name);

                        quote! {
                            pub #name: #account_ty
                        }
                    },
                ));

        tokens.extend(quote! {
            #[derive(Accounts)]
            #params
            pub struct #name<'info> { #(#accounts),* }
        })
    }
}

impl ToTokens for AccountTyExpr {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(match self {
            Self::Empty(ty_expr) => {
                quote! { #ty_expr }
            }
            Self::Defined(ty_expr) => {
                let ty_expr = StaticPath(ty_expr);

                quote! { Box<Account<'info, #ty_expr>> }
            }
            Self::Signer => quote! { Signer<'info> },
            Self::TokenMint => quote! { Box<Account<'info, Mint>> },
            Self::TokenAccount => quote! { Box<Account<'info, TokenAccount>> },
            Self::UncheckedAccount => quote! {
                UncheckedAccount<'info>
            },
            Self::SystemProgram => quote! { Program<'info, System> },
            Self::TokenProgram => quote! { Program<'info, Token> },
            Self::AssociatedTokenProgram => quote! { Program<'info, AssociatedToken> },
            Self::RentSysvar => quote! { Sysvar<'info, Rent> },
            Self::ClockSysvar => quote! { Sysvar<'info, Clock> },
        });
    }
}

/// The `space` parameter requires some extra info not present in the original annotation, so here's
/// my ugly solution.
struct AccountAnnotationWithTyExpr<'a>(&'a AccountAnnotation, &'a AccountTyExpr);
impl<'a> ToTokens for AccountAnnotationWithTyExpr<'a> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let AccountAnnotationWithTyExpr(
            AccountAnnotation {
                is_mut,
                is_associated,
                init,
                payer,
                seeds,
                token_mint,
                token_authority,
                mint_decimals,
                mint_authority,
                space,
                padding,
            },
            ty_expr,
        ) = self;

        // Build a filter-map of all the possible account attributes that Seahorse supports
        let mut params = vec![];

        if *is_mut {
            params.push(Some(quote! { mut }));
        }
        if *init {
            let ty_expr = match1!(ty_expr, AccountTyExpr::Empty(ty_expr) => ty_expr);
            if let AccountTyExpr::Defined(name) = &**ty_expr {
                let ty_expr = StaticPath(name);

                let space = match (space, padding) {
                    (None, None) => quote! { std::mem::size_of::<#ty_expr>() + 8 },
                    (Some(s), None) => quote! { #s as usize },
                    (None, Some(p)) => {
                        quote! { std::mem::size_of::<#ty_expr>() + 8 + (#p as usize) }
                    }
                    (Some(_), Some(_)) => panic!(), // we protect against this in prelude.rs
                };

                params.push(Some(quote! { init, space = #space }));
            } else {
                params.push(Some(quote! { init }));
            }
        }

        params.push(payer.as_ref().map(|payer| quote! { payer = #payer }));
        params.push(
            seeds
                .as_ref()
                .map(|seeds| quote! { seeds = [#(#seeds),*], bump }),
        );
        params.push(
            mint_decimals
                .as_ref()
                .map(|decimals| quote! { mint::decimals = #decimals }),
        );
        params.push(
            mint_authority
                .as_ref()
                .map(|authority| quote! { mint::authority = #authority }),
        );
        params.push(token_mint.as_ref().map(|mint| {
            if !*is_associated {
                quote! { token::mint = #mint }
            } else {
                quote! { associated_token::mint = #mint }
            }
        }));
        params.push(token_authority.as_ref().map(|authority| {
            if !*is_associated {
                quote! { token::authority = #authority }
            } else {
                quote! { associated_token::authority = #authority }
            }
        }));

        let params = params.into_iter().filter_map(|param| param);

        let unchecked = if let &&AccountTyExpr::UncheckedAccount = ty_expr {
            Some(quote! {
                /// CHECK: This account is unchecked.
            })
        } else {
            None
        };

        tokens.extend(quote! {
            // dummy: __SEAHORSE_ACCOUNT_ANNOTATION_FORMATTER__![[[ #(#params),* ]]],
            #[account(#(#params),*)]
            #unchecked
        });
    }
}

impl ToTokens for Block {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self {
            body,
            implicit_return,
        } = self;

        tokens.extend(quote! {{
            // Statements own their semicolons
            #(#body)*
            #implicit_return
        }});
    }
}

impl ToTokens for Statement {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(match self {
            // Tuple declaration in regular assignment statements gets converted to (maybe multiple)
            // `let mut <var>;` statements followed by a non-let assignment of the tuple, instead of just
            // doing a let-assignment of the tuple. This is to prevent variables from accidentally
            // shadowing each other. For illustration, we might have a scenario like this:
            //
            // x = 1
            // if cond:
            //     (x, y) = (2, 3)
            //
            // Translating this to Rust could cause problems if done naively:
            //
            // let mut x = 1;
            // if cond {
            //     // `x` just got shadowed, not mutated!
            //     let (x, y) = (2, 3)
            // }
            //
            // So instead:
            //
            // let mut x = 1;
            // if cond {
            //     let mut y;
            //     (x, y) = (2, 3)
            // }
            Self::Let {
                undeclared,
                target,
                value,
            } => {
                let value = Grouped(value);

                match target {
                    LetTarget::Var { .. } => {
                        // No undeclared variables means that this statement is just an assignment
                        if undeclared.len() == 0 {
                            let target = target.as_immut();
                            quote! { #target = #value; }
                        } else {
                            quote! { let #target = #value; }
                        }
                    }
                    LetTarget::Tuple(..) => {
                        let target = target.as_immut();

                        if undeclared.len() == 0 {
                            quote! { #target = #value; }
                        } else {
                            let undeclared = undeclared.iter().map(|var| {
                                let var = ident(var);

                                quote! { mut #var }
                            });

                            quote! {
                                let (#(#undeclared),*);
                                #target = #value;
                            }
                        }
                    }
                }
            }
            Self::Assign { receiver, value } => {
                let value = Grouped(value);

                // TODO maybe can do a normal = assignment if there are no `borrow`s in the rval?
                quote! {
                    assign!(#receiver, #value);
                }
            }
            Self::Expression(expression) => {
                let expression = Grouped(expression);

                quote! { #expression; }
            }
            Self::Return(value) => {
                let value = value.as_ref().map(|value| Grouped(value));

                quote! { return #value; }
            }
            Self::Break => quote! { break; },
            Self::Continue => quote! { continue; },
            Self::Noop => quote! {},
            Self::AnchorRequire { cond, msg } => {
                let msg = Grouped(msg);

                quote! {
                    if ! #cond {
                        panic!(#msg);
                    }
                }
            }
            Self::If { cond, body, orelse } => {
                let cond = Grouped(cond);

                match orelse {
                    // TODO if the `orelse` block just contains a single if statement, can turn this into an `else if`
                    Some(orelse) => quote! { if #cond #body else #orelse },
                    None => quote! { if #cond #body },
                }
            }
            Self::While { cond, body } => {
                let cond = Grouped(cond);

                quote! { while #cond #body }
            }
            Self::Loop { label, body } => {
                let label = match label {
                    Some(label) => Some(ident(label)),
                    None => None,
                };

                quote! { #label loop #body }
            }
            Self::For { target, iter, body } => {
                let iter = Grouped(iter);

                quote! { for #target in #iter #body }
            }
        });
    }
}

impl ToTokens for LetTarget {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(match self {
            Self::Var { name, is_mut } => {
                let name = ident(name);

                match is_mut {
                    true => quote! { mut #name },
                    false => quote! { #name },
                }
            }
            Self::Tuple(targets) => quote! { (#(#targets),*) },
        });
    }
}

impl ToTokens for TypedExpression {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self { obj, .. } = self;

        tokens.extend(quote! { #obj });

        // let Self { ty, obj } = self;
        // let ty_str = format!("{}", ty);
        // tokens.extend(quote! { ty!(#ty_str, #obj) });
    }
}

/// Newtype for an expression that doesn't need extra parentheses to group it.
struct Grouped<'a>(&'a TypedExpression);
impl<'a> ToTokens for Grouped<'a> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(match &self.0.obj {
            ExpressionObj::BinOp { left, op, right } => quote! { #left #op #right },
            ExpressionObj::UnOp { op, value } => quote! { #op #value },
            ExpressionObj::As { value, ty } => {
                let ty = LoadedTyExpr(ty);

                quote! { #value as #ty }
            }
            obj => quote! { #obj },
        });
    }
}

impl ToTokens for ExpressionObj {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(match self {
            Self::BinOp { left, op, right } => quote! { (#left #op #right) },
            Self::Index { value, index } => {
                let value = Grouped(&**value);
                let index = Grouped(&**index);

                quote! { #value[#index] }
            }
            Self::TupleIndex { tuple, index } => {
                let index = PM2Literal::usize_unsuffixed(*index);

                quote! { #tuple . #index }
            }
            Self::UnOp { op, value } => quote! { (#op #value) },
            Self::Attribute { value, name } => {
                let name = ident(name);

                quote! { #value . #name }
            }
            Self::StaticAttribute { value, name } => {
                let name = ident(name);

                quote! { #value :: #name }
            }
            Self::Call { function, args } => {
                let args = args.iter().map(|arg| Grouped(arg));

                quote! { #function(#(#args),*) }
            }
            Self::Ternary { cond, body, orelse } => {
                let cond = Grouped(&**cond);

                quote! {
                    if #cond { #body } else { #orelse }
                }
            }
            Self::As { value, ty } => {
                let ty = LoadedTyExpr(ty);

                quote! { (#value as #ty) }
            }
            Self::Vec(elements) => {
                let elements = elements.iter().map(|element| Grouped(element));

                quote! { vec![#(#elements),*] }
            }
            Self::Array(elements) => {
                let elements = elements.iter().map(|element| Grouped(element));

                quote! { [#(#elements),*] }
            }
            Self::Tuple(tuple) => {
                let tuple = tuple.iter().map(|part| Grouped(part));

                quote! { (#(#tuple),*) }
            }
            Self::Id(name) => {
                let name = ident(name);

                quote! { #name }
            }
            Self::Literal(literal) => quote! { #literal },
            Self::Block(block) => quote! { #block },
            Self::Ref(value) => quote! { (& #value) },
            Self::Move(value) => {
                // If we're trying to move some data, we need to explicitly
                // clone it if the type is not `Copy` and the data is owned
                if !value.ty.is_copy() && value.obj.is_owned() {
                    quote! { #value . clone() }
                } else {
                    quote! { #value }
                }
            }
            Self::BorrowMut(value) => quote! { #value . borrow_mut() },
            Self::BorrowImmut(value) => quote! { #value . borrow() },
            Self::Mutable(value) => {
                let value = Grouped(&**value);

                quote! { Mutable::new(#value) }
            }
            Self::Rendered(tokens) => tokens.clone(),
            Self::Placeholder => panic!("Attempted to convert an explicit placeholder to tokens"),
        });
    }
}

impl ToTokens for Literal {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(match self {
            Self::Int(n) => {
                let n = PM2Literal::i128_unsuffixed(*n);

                quote! { #n }
            }
            Self::Float(n) => quote! { #n },
            Self::Str(s) => quote! { #s },
            Self::Bool(p) => quote! { #p },
            Self::Unit => quote! { () },
        });
    }
}

impl ToTokens for Operator {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(match self {
            Self::Add => quote! { + },
            Self::Sub => quote! { - },
            Self::Mul => quote! { * },
            Self::Div => quote! { / },
            Self::Mod => quote! { % },
            // Self::Pow => quote! { + },
            Self::LShift => quote! { << },
            Self::RShift => quote! { >> },
            Self::BitOr => quote! { | },
            Self::BitXor => quote! { ^ },
            Self::BitAnd => quote! { & },
            Self::And => quote! { && },
            Self::Or => quote! { || },
            Self::Eq => quote! { == },
            Self::NotEq => quote! { != },
            Self::Lt => quote! { < },
            Self::Lte => quote! { <= },
            Self::Gt => quote! { > },
            Self::Gte => quote! { >= },
        })
    }
}

impl ToTokens for UnaryOperator {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(match self {
            Self::Pos => quote! { + },
            Self::Neg => quote! { - },
            Self::Not => quote! { ! },
            Self::Inv => quote! { ! },
        })
    }
}

fn make_lib(
    origin: &Artifact,
    path: &Vec<String>,
    program_name: &String,
) -> CResult<String> {
    let program_name = ident(program_name);

    let mut id = None;
    for directive in origin.directives.iter() {
        match directive {
            Directive::DeclareId(id_str) => {
                id = Some(id_str.clone());
            } // _ => {}
        }
    }

    if id.is_none() {
        return Err(CoreError::make_raw(
            "declare_id not found",
            "Help: Anchor should generate your program's ID, check the IDL for this program in target/idl/<program name>.json, and add it to your program:\n\n    declare_id(\"id from the .json file\")"
        ));
    }

    let instructions = origin.functions.iter().filter_map(
        |Function {
             name,
             ix_context,
             params,
             ..
         }| {
            let ix_context = match ix_context {
                Some(ix_context) => ix_context,
                None => {
                    return None;
                }
            };

            let name = ident(name);
            let handler_name = ident(&format!("{}_handler", name));
            let context_name = ident(&ix_context.name);

            let insert_programs = ix_context.inferred_accounts.iter().filter_map(
                |(name, ContextAccount { account_ty, .. })| {
                    if account_ty.is_program() {
                        let name_id = ident(&String::from(name));

                        Some(quote! {
                            programs.insert(#name, ctx.accounts.#name_id.to_account_info());
                        })
                    } else {
                        None
                    }
                },
            );

            let load_accounts = ix_context.accounts.iter().filter_map(
                |(name, ContextAccount { account_ty, ty, .. })| {
                    let name = ident(name);

                    // Deconstruct `Empty` accounts
                    let (is_empty, account_ty) = match account_ty {
                        AccountTyExpr::Empty(empty) => (true, &**empty),
                        ty => (false, ty),
                    };

                    let loaded = match account_ty {
                        AccountTyExpr::Defined(path) => {
                            let path = StaticPath(path);

                            quote! { #path::load(&mut ctx.accounts.#name, &programs_map) }
                        }
                        AccountTyExpr::Signer => quote! {
                            SeahorseSigner {
                                account: &ctx.accounts.#name,
                                programs: &programs_map
                            }
                        },
                        AccountTyExpr::TokenMint | AccountTyExpr::TokenAccount => quote! {
                            SeahorseAccount {
                                account: &ctx.accounts.#name,
                                programs: &programs_map
                            }
                        },
                        AccountTyExpr::UncheckedAccount => quote! {
                            &ctx.accounts.#name.clone()
                        },
                        AccountTyExpr::ClockSysvar => quote! {
                            &ctx.accounts.#name.clone()
                        },
                        _ => {
                            return None;
                        }
                    };

                    Some(if is_empty {
                        quote! {
                            let #name = Empty {
                                account: #loaded,
                                bump: Some(ctx.bumps.#name)
                            };
                        }
                    } else {
                        quote! {
                            let #name = #loaded;
                        }
                    })
                },
            );

            let ix_params = params.iter().filter_map(|(name, ty)| {
                if !ix_context.params.iter().any(|(name_, _)| name == name_) {
                    return None;
                }

                let name = ident(name);
                let ty = StoredTyExpr(ty);

                Some(quote! { #name: #ty })
            });

            let params = params.iter().map(|(name, _)| {
                let name = ident(name);

                // TODO don't need to clone all accounts
                if ix_context.params.iter().any(|(name_, _)| name == name_) {
                    quote! { #name }
                } else {
                    quote! { #name.clone() }
                }
            });

            let store_accounts = ix_context.accounts.iter().filter_map(
                |(name, ContextAccount { account_ty, .. })| {
                    // Deconstruct `Empty` accounts
                    let (is_empty, account_ty) = match account_ty {
                        AccountTyExpr::Empty(empty) => (true, &**empty),
                        ty => (false, ty),
                    };

                    match account_ty {
                        AccountTyExpr::Defined(path) => {
                            let name = ident(name);
                            let path = StaticPath(path);

                            if is_empty {
                                Some(quote! { #path::store(#name.account); })
                            } else {
                                Some(quote! { #path::store(#name); })
                            }
                        }
                        _ => None,
                    }
                },
            );

            Some(quote! {
                #ix_context

                pub fn #name(ctx: Context<#context_name>, #(#ix_params),*) -> Result<()> {
                    let mut programs = HashMap::new();
                    #(#insert_programs)*
                    let programs_map = ProgramsMap(programs);

                    #(#load_accounts)*
                    #handler_name(#(#params),*);
                    #(#store_accounts)*

                    return Ok(());
                }
            })
        },
    );

    let path = StaticPath(path);

    let text = beautify(quote! {
        use std::{cell::RefCell, rc::Rc};
        use anchor_lang::prelude::*;
        use anchor_spl::{
            token::{self, Mint, Token, TokenAccount},
            associated_token::{self, AssociatedToken}
        };
        use #path::*;

        declare_id!(#id);

        mod seahorse_util;
        use seahorse_util::*;

        #[program]
        mod #program_name {
            use super::*;
            use seahorse_util::*;
            use std::collections::HashMap;

            #(#instructions)*
        }
    })?;

    return Ok(text);
}

/// Add mod.rs content.
fn add_mods(tree: &mut Tree<String>) {
    match tree {
        Tree::Node(node) => {
            let mods = node.keys().map(|key| {
                let key = ident(key);

                quote! { pub mod #key; }
            });
            let text = beautify(quote! { #(#mods)* }).unwrap();
            node.insert("mod".to_string(), Tree::Leaf(text));

            for (_, tree) in node.iter_mut() {
                add_mods(tree);
            }
        }
        _ => {}
    }
}

// Rustfmt isn't supported for wasm
#[cfg(not(target_arch = "wasm32"))]
/// Make a `TokenStream` look nice.
fn beautify_impl(tokens: TokenStream) -> CResult<String> {
    let config = Config {
        // Maybe there will be something here one day
        ..Config::default()
    };

    let mut source = rustfmt_config(config, tokens).map_err(|err| match err {
        RustfmtError::NoRustfmt => CoreError::make_raw(
            "rustfmt not installed",
            "Help: Seahorse depends on rustfmt, which is part of the Rust toolchain. To install:\n\n    rustup components add rustfmt"
        ),
        RustfmtError::Rustfmt(message) => CoreError::make_raw(
            "rustfmt error",
            format!("{}This is most likely an error in Seahorse.", message)
        ),
        _ => CoreError::make_raw("unknown rustfmt error", ""),
    })?;

    // Perform some simple regex-based transformations
    // NOTE makes some bold assumptions about the spacing in rustfmt's output. Likely won't cause
    // any major problems, at worst has the potential to corrupt very weird strings

    // let re = Regex::new(r"(?s)dummy: __SEAHORSE_ACCOUNT_ANNOTATION_FORMATTER__\s*!\s*\[\[\[(.*?)\]\]\],").unwrap();
    // source = re.replace_all(&source, "#[account($1)]").to_string();

    // Add a blank line after the end of statements or blocks (semicolon or right curly at line
    // end), but not before the end of a block
    let re = Regex::new(r"([};])\n(\s*[^\s}])").unwrap();
    source = re.replace_all(&source, "$1\n\n$2").to_string();

    // Remove blank lines between blocks of (single-line) "use ...;"
    // Regex::replace_all only catches non-overlapping occurrences of the pattern, so we have to
    // run this twice to get everything
    let re = Regex::new(r"(use .*?;)\n\n(\s*use )").unwrap();
    source = re.replace_all(&source, "$1\n$2").to_string();
    let re = Regex::new(r"(use .*?;)\n\n(\s*use )").unwrap();
    source = re.replace_all(&source, "$1\n$2").to_string();

    // Remove blank lines between blocks of (single-line) "let ...;"
    // ^ ditto with use statements, run this twice to catch everything
    let re = Regex::new(r"(let .*?;)\n\n(\s*let )").unwrap();
    source = re.replace_all(&source, "$1\n$2").to_string();
    let re = Regex::new(r"(let .*?;)\n\n(\s*let )").unwrap();
    source = re.replace_all(&source, "$1\n$2").to_string();

    // Collapse any accidental doubled blank lines
    let re = Regex::new(r"\n\n\n+").unwrap();
    source = re.replace_all(&source, "\n\n").to_string();

    // Convert #[doc = "CHECK: ..."] back to /// CHECK: ...
    let re = Regex::new(r#"#\[doc = r" CHECK: (.*?)"\]"#).unwrap();
    source = re.replace_all(&source, "/// CHECK: $1").to_string();

    // rustfmt misses some stuff around attributes, so we'll fix that here
    // Remove spaces around double colons
    let re = Regex::new(r"\s*::\s*").unwrap();
    source = re.replace_all(&source, "::").to_string();

    // Remove spaces in annotation name, like # [ account and # [ instruction
    let re = Regex::new(r"#\s*\[\s*(\w+) \(").unwrap();
    source = re.replace_all(&source, "#[$1(").to_string();

    // Remove spaces before commas
    let re = Regex::new(r"\s+,").unwrap();
    source = re.replace_all(&source, ",").to_string();

    // Remove space before colon (no ternary operator in Rust)
    let re = Regex::new(r"\s+:\s").unwrap();
    source = re.replace_all(&source, ": ").to_string();

    // Remove spaces around the contents of angle brackets, so < foo > to <foo>
    let re = Regex::new(r"<\s*(.*?)\s*>").unwrap();
    source = re.replace_all(&source, "<$1>").to_string();

    // Remove spaces around periods
    let re = Regex::new(r"\s+\.\s+").unwrap();
    source = re.replace_all(&source, ".").to_string();

    // Remove spaces between a word and (), i.e. fn calls
    let re = Regex::new(r"(\w|)\>*\s*\(").unwrap();
    source = re.replace_all(&source, "$1(").to_string();

    Ok(source)
}

fn beautify(tokens: TokenStream) -> CResult<String> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        return beautify_impl(tokens);
    }
    #[cfg(target_arch = "wasm32")]
    {
        return Ok(tokens.to_string());
    }
}

impl TryFrom<(BuildOutput, String)> for GenerateOutput {
    type Error = CoreError;

    fn try_from((build_output, program_name): (BuildOutput, String)) -> CResult<Self> {
        let tree = build_output.tree.clone();
        let origin = tree.get_leaf(&build_output.origin).unwrap();

        let features = Rc::new(RefCell::new(BTreeSet::new()));

        let mut tree = build_output
            .tree
            .map(|artifact| {
                let text = beautify(quote! { #artifact })?;

                features.borrow_mut().extend(artifact.features.into_iter());

                Ok(text)
            })
            .transpose()?;

        let features = features.take();
        let maybe_pyth_import = if features.contains(&Feature::Pyth) {
            Some(quote! {
                // Re-export for ease of access
                pub use pyth_sdk_solana::{load_price_feed_from_account_info, PriceFeed};
            })
        } else {
            None
        };
        let lib = make_lib(origin, &build_output.origin, &program_name)?;

        add_mods(&mut tree);

        if let Tree::Node(node) = &mut tree {
            let allows = concat!(
                "#![allow(unused_imports)]\n",
                "#![allow(unused_variables)]\n",
                "#![allow(unused_mut)]\n"
            );

            let mod_text = match node.remove("mod") {
                Some(Tree::Leaf(text)) => text,
                _ => panic!(),
            };

            node.insert(
                "lib".to_string(),
                Tree::Leaf(format!("{}\n{}\n{}", allows, mod_text, lib)),
            );

            node.insert(
                "seahorse_util".to_string(),
                Tree::Leaf(
                    format!(
                        "{}\n{}\n{}",
                        allows,
                        maybe_pyth_import.unwrap_or_default(),
                        SEAHORSE_UTIL
                    )
                )
            );
        }

        return Ok(GenerateOutput { tree, features });
    }
}

pub fn generate(build_output: BuildOutput, program_name: String) -> CResult<GenerateOutput> {
    (build_output, program_name).try_into()
}
