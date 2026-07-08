use proc_macro2::TokenStream;
use quote::quote;
use syn::{LitStr, ext::IdentExt};

pub(crate) fn create_from_row_for(
    base_struct: &super::base_struct::StructData,
    tokens: &mut proc_macro2::TokenStream,
) {
    create_from_row_for_base_struct(base_struct, tokens);
    create_from_row_for_base_struct(&base_struct.to_partial(), tokens);
}

pub(crate) fn create_from_row_for_base_struct(
    base_struct: &super::base_struct::StructData,
    tokens: &mut proc_macro2::TokenStream,
) {
    let a = &base_struct.generics;
    let b = &base_struct.generics_names_only(TokenStream::new());
    let c = &base_struct.where_clause;

    let name = &base_struct.name;
    let from_row_body = if base_struct.is_partial {
        create_try_from_row_body(base_struct)
    } else {
        let partial = base_struct.partial_name();
        quote!(
            use silo::partial::PartialType;
            #partial::try_from_row(row, connection)?.transpose().ok_or(silo::Error::Todo("Improve error handling here, so we know which column was missing".into()))
        )
    };
    let from_row = quote! {
        impl #a silo::FromRow for #name #b #c {
            fn try_from_row(
                row: &silo::rusqlite::Row,
                connection: &silo::rusqlite::Connection,
            ) -> std::result::Result<Self, silo::Error> {
                #from_row_body
            }
        }
    };
    tokens.extend(from_row);
}

fn create_try_from_row_body(
    base_struct: &super::base_struct::StructData,
) -> proc_macro2::TokenStream {
    let columns = base_struct.columns();
    let column_names: Vec<syn::Ident> = columns.iter().map(|c| c.ident()).collect();
    let column_names_str_lit = column_names.iter().map(|c| {
        let n = c.unraw();
        LitStr::new(&n.to_string(), n.span())
    });
    let column_types = columns.iter().map(|c| c.type_);
    let column_default_values = columns.iter().map(|c| {
        let n = c.ident();
        c.default_value
            .as_ref()
            .map(|d| {
                quote! {
                    let #n = #n.or(#d);
                }
            })
            .unwrap_or_default()
    });

    if let Some(_variant) = base_struct.variant_field().map(|f| f.name) {
        quote! {todo!("Enums not yet supported!")}
    } else {
        quote! {#(
            let #column_names = <#column_types as silo::ExtractFromRow>::try_from_row(#column_names_str_lit, row, connection)?;
            #column_default_values
        )*
        Ok(Self {
            #(#column_names,)*
        })}
    }
}
