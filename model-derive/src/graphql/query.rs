//! Derive macro for the top-level GraphQL queries over an ontology of `Resource` types.

use super::graphql_path;
use crate::helpers::{parse_docs, AttrParser};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Attribute, Data, DeriveInput, Field, Ident};

/// Generate a GraphQL entrypoint for a query struct.
pub fn derive(
    DeriveInput {
        ident,
        generics,
        data,
        attrs,
        ..
    }: DeriveInput,
) -> TokenStream {
    if !generics.params.is_empty() {
        panic!("Query cannot be derived on generic types");
    }
    match data {
        Data::Struct(_) => generate_struct(ident, attrs),
        _ => panic!("Query can only be derived for structs"),
    }
}

fn generate_struct(name: Ident, attrs: Vec<Attribute>) -> TokenStream {
    let graphql = graphql_path();
    let p = AttrParser::new("query");

    // We will generate the impl in an anonymous module so we can bring things into scope.
    let mod_name = format_ident!("__query_object_{}", name);

    // Get the documentation from the original struct. We will attach it to the `#[Object]` impl
    // block so it shows up in the exported schema.
    let doc = parse_docs(&attrs);

    // Get the list of resources in the ontology.
    let resources = attrs
        .iter()
        .flat_map(|a| p.parse_arg_with(a, "resource", Field::parse_named));

    // Generate resolvers for each resource.
    let resolvers = resources.map(|resource| {
        let name = resource.ident.unwrap();
        let ty = resource.ty;
        let doc = format!("Search for {}.", name);
        quote! {
            #[doc = #doc]
            async fn #name(
                &self,
                #[graphql(name = "where")]
                pred: Option<<#ty as Type>::Predicate>,
                after: Option<String>,
                before: Option<String>,
                first: Option<usize>,
                last: Option<usize>,
            ) -> Result<Connection<Cursor<D, #ty>, #ty>> {
                todo!()
            }
        }
    });

    quote! {
        #[allow(non_snake_case)]
        mod #mod_name {
            use super::*;
            use #graphql::{
                async_graphql, connection::Connection, backend::{Cursor, DataSource},
                type_system::Type, D, EmptyFields, Result,
            };

            #[Object]
            #[doc = #doc]
            impl #name {
                #(#resolvers)*
            }
        }
    }
}
