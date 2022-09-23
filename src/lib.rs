//! A `Selectable` derive trait for deriving a `select` method on a struct
//! that performs a Diesel query by key names, rather than position (as
//! [`diesel::Queryable`] does).

use darling::ast;
use darling::FromDeriveInput;
use darling::FromField;
use darling::FromMeta;
use proc_macro::TokenStream as TokenStream1;
use proc_macro2::TokenStream;
use quote::quote;
use quote::ToTokens;
use syn::parse_macro_input;
use syn::Attribute;
use syn::DeriveInput;
use syn::Generics;
use syn::Ident;

/// Provide a `.select()` function based on the struct's fields.
#[proc_macro_derive(Selectable, attributes(table_name))]
#[cfg(not(tarpaulin_include))]
pub fn derive_selectable(input: TokenStream1) -> TokenStream1 {
  SelectableStruct::from_derive_input(&parse_macro_input!(
    input as DeriveInput
  ))
  .map(|recv| quote!(#recv))
  .unwrap_or_else(|err| err.write_errors())
  .into()
}

/// Struct that receives the input struct to `Selectable` and augments it
/// with the `.select()` function.
#[derive(FromDeriveInput)]
#[darling(supports(struct_named), forward_attrs(table_name))]
pub(crate) struct SelectableStruct {
  /// The name of the struct.
  ident: Ident,

  // Lifetimes and type parameters attached to the struct.
  generics: Generics,

  /// Data on the individual fields.
  data: ast::Data<(), SelectableField>,

  /// Attributes on the overall struct.
  attrs: Vec<Attribute>,
}

impl SelectableStruct {
  /// Return the field identifiers on the struct.
  #[cfg(not(tarpaulin_include))]
  fn field_names(&self) -> Vec<&Ident> {
    self
      .data
      .as_ref()
      .take_struct()
      .expect("Selectable only supports named structs")
      .into_iter()
      .map(|field| field.name())
      .collect()
  }
}

impl ToTokens for SelectableStruct {
  /// Return an automatically generated `Selectable` implementation.
  #[cfg(not(tarpaulin_include))]
  fn to_tokens(&self, tokens: &mut TokenStream) {
    // Put together our basic tokens: the struct identifier, and any generics
    // (types, lifetimes, etc.) we need to carry forward to the `Selectable`
    // implementation.
    let ident = &self.ident;
    let (impl_generics, type_generics, where_clause) =
      self.generics.split_for_impl();

    // Get the name of the table.
    let meta = self
      .attrs
      .iter()
      .find(|attr| attr.path.is_ident("table"))
      .expect("The `table` attribute is required")
      .parse_meta()
      .expect("Unable to parse `table` attribute");
    let table_name: Ident = Ident::from_meta(&meta).expect("Bad identifier");
    let table = quote! { crate::schema::#table_name::dsl::#table_name };

    // Get the list of fields as a tuple.
    let fields: Vec<TokenStream> = self
      .field_names()
      .iter()
      .map(|f| quote! { crate::schema::#table_name::dsl::#f })
      .collect();

    // Add the select implementation.
    tokens.extend(quote! {
      #[automatically_derived]
      impl #impl_generics #ident #type_generics #where_clause {
        /// Construct a query object to retrieve objects from the corresponding
        /// database table.
        pub fn select() -> diesel::dsl::Select<#table, (#(#fields),*)> {
          #table.select((#(#fields),*))
        }
      }
    })
  }
}

/// A representation of a single field on the struct.
#[derive(FromField)]
#[darling(attributes(field_names))]
struct SelectableField {
  /// The name of the field, or None for tuple fields.
  ident: Option<Ident>,
}

impl SelectableField {
  /// Return the field's identifier, or panic if there is no identifier.
  #[cfg(not(tarpaulin_include))]
  fn name(&self) -> &Ident {
    self.ident.as_ref().expect("Selectable only supports named fields")
  }
}
