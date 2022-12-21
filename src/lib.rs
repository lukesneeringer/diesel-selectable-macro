//! A `Selectable` derive trait for deriving a `select` method on a struct
//! that performs a Diesel query by key names, rather than position (as
//! [`diesel::Queryable`] does).

use std::ops::Deref;

use darling::ast;
use darling::FromDeriveInput;
use darling::FromField;
use proc_macro::TokenStream as TokenStream1;
use proc_macro2::TokenStream;
use quote::quote;
use quote::ToTokens;
use syn::parse::Parse;
use syn::parse::ParseStream;
use syn::parse_macro_input;
use syn::Attribute;
use syn::DeriveInput;
use syn::Generics;
use syn::Ident;

/// Provide a `.select()` function based on the struct's fields.
#[proc_macro_derive(Selectable, attributes(diesel))]
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
#[darling(supports(struct_named), forward_attrs(diesel))]
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
    let diesel = self
      .attrs
      .iter()
      .find(|attr| attr.path.is_ident("diesel"))
      .expect("The `diesel` attribute is required");
    let args = syn::parse_macro_input::parse::<CommaSeparatedArguments>(
      diesel.into_token_stream().into(),
    )
    .expect("Unable to parse arguments.");
    let table_name = args
      .iter()
      .find_map(|arg| {
        syn::parse_macro_input::parse::<TableNameParser>(arg.clone().into())
          .ok()
      })
      .expect("No `table_name` argument found.")
      .table_name;
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
        /// Return a tuple of the table's fields.
        pub fn fields() -> (#(#fields),*) {
          (#(#fields),*)
        }

        /// Construct a query object to retrieve objects from the corresponding
        /// database table.
        pub fn select() -> diesel::dsl::Select<#table, (#(#fields),*)> {
          #table.select(Self::fields())
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

struct CommaSeparatedArguments(Vec<TokenStream>);

impl Parse for CommaSeparatedArguments {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    let bracketed;
    let content;
    input.parse::<syn::Token![#]>()?;
    syn::bracketed!(bracketed in input);
    bracketed.parse::<syn::Ident>()?;
    syn::parenthesized!(content in bracketed);

    // There are zero or more arguments, comma separated. Split them up.
    Ok(Self(
      content
        .parse_terminated::<TokenStream, syn::Token![,]>(TokenStream::parse)
        .expect("Failed to parse comma-separated args")
        .into_iter()
        .collect(),
    ))
  }
}

impl Deref for CommaSeparatedArguments {
  type Target = Vec<TokenStream>;

  fn deref(&self) -> &Self::Target {
    &self.0
  }
}

struct TableNameParser {
  table_name: Ident,
}

impl Parse for TableNameParser {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    // We're looking for `table_name = foo`, so first we can split on `=` and
    // collect the result; the length should be 2 if this is a match.
    let key_val: Vec<Ident> = input
      .parse_terminated::<Ident, syn::Token![=]>(Ident::parse)
      .expect("Not = separated.")
      .into_iter()
      .collect();
    if key_val.len() != 2 {
      return Err(input.error("Incorrect token length."));
    }
    match key_val[0] == "table_name" {
      true => Ok(Self { table_name: key_val[1].clone() }),
      false => Err(input.error("Wrong attribute,")),
    }
  }
}
