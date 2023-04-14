//! Derive macro for the `Class` trait.

use super::graphql_path;
use crate::helpers::{parse_docs, AttrParser};
use convert_case::{Case, Casing};
use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use syn::{
    Attribute, Data, DataStruct, DeriveInput, Field, Fields, Ident, LitBool, Type, Visibility,
};

/// Derive a `Class` instance for a struct.
pub fn derive(
    DeriveInput {
        vis,
        ident,
        generics,
        data,
        attrs,
    }: DeriveInput,
) -> TokenStream {
    if !generics.params.is_empty() {
        panic!("Class cannot be derived on generic types");
    }
    match data {
        Data::Struct(s) => generate_struct(s, vis, ident, attrs),
        _ => panic!("Class can only be derived for structs"),
    }
}

fn generate_struct(
    s: DataStruct,
    vis: Visibility,
    name: Ident,
    attrs: Vec<Attribute>,
) -> TokenStream {
    let graphql = graphql_path();
    let p = AttrParser::new("class");

    // Get fields, ignoring skipped ones.
    let Fields::Named(fields) = s.fields else {
        panic!("Class fields must be named");
    };
    let fields = fields
        .named
        .into_iter()
        .filter(|f| !p.has_bool(&f.attrs, "skip"))
        .collect::<Vec<_>>();

    // Get the plural name for this struct. If it is given explicitly via the `plural` attribute we
    // will use that; otherwise just add an `s`.
    let plural_name = p
        .get_arg(&attrs, "plural")
        .unwrap_or_else(|| format_ident!("{}s", name));

    // Derive a name for the module that will contain the generated items;
    let mod_name = p
        .get_arg(&attrs, "module")
        .unwrap_or_else(|| Ident::new(&name.to_string().to_case(Case::Snake), Span::call_site()));

    // Get the documentation on this struct. We will need to add this to the generated `#[Object]`
    // impl so that it shows up in the exported schema.
    let doc = parse_docs(&attrs);

    // Derive names for the various predicate structs we are going to create.
    let pred_name = format_ident!("{}Predicate", name);
    let has_name = format_ident!("{}Has", name);
    let plural_pred_name = format_ident!("{}Predicate", plural_name);
    let quant_name = format_ident!("Quantified{}Predicate", name);

    // Create documentation for each of the predicate structs.
    let has_doc = format!("A predicate on fields of {}.", name);
    let pred_doc = format!("A predicate used to filter {}.", plural_name);
    let quant_doc = format!(
        "A predicate which must match a certain quantity of {}.",
        plural_name
    );
    let plural_pred_doc = format!("A predicate used to filter collections of {}.", plural_name);

    // Generate predicates for each field of this struct.
    let pred_fields = fields.iter().map(|f| generate_predicate_field(&p, f));

    // If this struct has a _primary field_, it gets a couple of extra predicate options. In
    // addition to filtering by applying a predicate to its fields, you can implicitly filter based
    // on the primary field, which leads to shorter, more readable queries. In particular, you can
    // say, e.g. `state CA` instead of `state with abbreviation CA`.
    let primary_field = fields.iter().find(|f| p.has_bool(&f.attrs, "primary"));
    // The `is` predicate based on the primary field.
    let is_primary = primary_field.as_ref().map(|f| {
        let ty = &f.ty;
        quote! {
            /// Filter by value.
            Is(<#ty as Class>::Predicate),
        }
    });
    // The `includes` plural predicate, filtering a collection based on whether or not it contains
    // any items with the given value for their primary field.
    let includes_primary = primary_field.as_ref().map(|f| {
        let ty = &f.ty;
        quote! {
            /// Matches if the collection includes the specified value.
            Includes(Value<#ty>),
        }
    });

    // Generate resolvers for each field.
    let resolvers = fields.iter().map(|f| generate_resolver(&p, f));

    quote! {
        #vis mod #mod_name {
            use super::*;
            use #graphql::{
                async_graphql, connection::Connection, scalars::Value, traits::{DataSource, Many},
                Class, Context, D, EmptyFields, InputObject, Object, OneofObject, Plural, Result,
            };

            #[doc = #doc]
            #[Object]
            impl #name {
                #(#resolvers)*
            }

            #[doc = #has_doc]
            #[derive(Clone, Debug, InputObject)]
            pub struct #has_name {
                #(#vis #pred_fields),*
            }

            #[doc = #pred_doc]
            #[derive(Clone, Debug, OneofObject)]
            pub enum #pred_name {
                /// Filter by fields.
                Has(Box<#has_name>),
                #is_primary
            }

            #[doc = #quant_doc]
            #[derive(Clone, Debug, InputObject)]
            pub struct #quant_name {
                /// The minimum or maximum number of items which must match.
                #vis quantity: usize,
                /// The predicate to match against specific items.
                #vis predicate: #pred_name,
            }

            #[doc = #plural_pred_doc]
            #[derive(Clone, Debug, OneofObject)]
            pub enum #plural_pred_name {
                /// Matches if at least some number of items in the collection match a predicate.
                AtLeast(#quant_name),
                /// Matches if at most some number of items in the collection match a predicate.
                AtMost(#quant_name),
                /// Matches if at any items in the collection match a predicate.
                Any(#pred_name),
                /// Matches if all items in the collection match a predicate.
                All(#pred_name),
                /// Matches if no items in the collection match a predicate.
                None(#pred_name),
                #includes_primary
            }

            impl Class for #name {
                type Plural = Many<D, Self>;
                type Predicate = #pred_name;
                type PluralPredicate = #plural_pred_name;
            }
        }
    }
}

fn generate_predicate_field(p: &AttrParser, f: &Field) -> TokenStream {
    let ty = &f.ty;
    let Some(name) = &f.ident else {
        panic!("Class fields must be named");
    };
    if field_is_plural(p, f) {
        // If the field is plural, it is filtered by a predicate on collections of the singular
        // type.
        quote! {
            #name: Option<<<#ty as Plural>::Singular as Class>::PluralPredicate>
        }
    } else {
        // Otherwise it is just filtered by the regular singular predicate.
        quote! {
            #name: Option<<#ty as Class>::Predicate>
        }
    }
}

fn generate_resolver(p: &AttrParser, f: &Field) -> TokenStream {
    let name = &f.ident;
    let ty = &f.ty;
    let doc = parse_docs(&f.attrs);

    if field_is_plural(p, f) {
        let singular = quote! { <#ty as Plural>::Singular };
        quote! {
            // Resolvers that yields collections get extra parameters for filtering the collections,
            // including paging parameters and a where clause.
            #[doc = #doc]
            async fn #name(
                &self,
                ctx: &Context<'_>,
                #[graphql(name = "where")]
                filter: Option<<#singular as Class>::Predicate>,
                after: Option<String>,
                before: Option<String>,
                first: Option<usize>,
                last: Option<usize>,
            ) -> Result<
                    Connection<
                        <D as DataSource>::Cursor<
                            #singular, EmptyFields
                        >,
                        #singular,
                    >
                >
            {
                // Use the corresponding field to silence dead code warnings. We can remove this
                // once we have actually implemented this resolver.
                let _ = &self.#name;

                todo!()
            }
        }
    } else {
        quote! {
            // Singular fields just resolve to the field itself.
            #[doc = #doc]
            async fn #name(&self) -> &#ty {
                &self.#name
            }
        }
    }
}

fn field_is_plural(p: &AttrParser, f: &Field) -> bool {
    // Check if the field is explicitly plural.
    if p.has_bool(&f.attrs, "plural") {
        return true;
    }
    let explicit: Option<LitBool> = p.get_arg(&f.attrs, "plural");
    let explicit = explicit.map(|lit| lit.value);
    if explicit == Some(true) {
        return true;
    }

    // Check if the field has an implicitly plural type (e.g. `Many`). If not, it is not plural.
    let Type::Path(path) = &f.ty else { return false; };
    let Some(type_name) = path.path.segments.last() else { return false; };
    if type_name.ident == "Many" && explicit != Some(false) {
        return true;
    }
    // If the field is not explicitly or implicitly plural, it is not plural.
    false
}
