use quote::{format_ident, quote};
use syn::Ident;

pub(crate) fn create_from_row_for(
    base_struct: &crate::base_struct::StructData,
    tokens: &mut proc_macro2::TokenStream,
) {
    create_from_row_for_base_struct(base_struct, tokens);
    create_from_row_for_base_struct(&base_struct.to_partial(), tokens);
}

pub(crate) fn create_from_row_for_base_struct(
    base_struct: &crate::base_struct::StructData,
    tokens: &mut proc_macro2::TokenStream,
) {
    let name = &base_struct.name;
    let from_row_body = if base_struct.is_partial {
        create_try_from_row_body(base_struct)
    } else {
        let partial = base_struct.partial_name();
        quote!(
            use silo::PartialType;
            #partial::try_from_row(row, connection)?.transpose().ok_or(silo::Error::Todo("Improve error handling here, so we know which column was missing".into()))
        )
    };
    let extract_from_row_body = if base_struct.is_partial {
        create_extract_from_row_body(&base_struct.original_name, base_struct)
    } else {
        let partial = base_struct.partial_name();
        quote!(
            use silo::PartialType;
            #partial::try_from_row(column, row, connection)?.transpose().ok_or(silo::Error::Todo("Improve error handling here, so we know which column was missing".into()))
        )
    };
    let from_row = quote! {
        impl silo::FromRow for #name {
            fn try_from_row(
                row: &silo::rusqlite::Row,
                connection: &silo::rusqlite::Connection,
            ) -> Result<Self, silo::Error> {
                #from_row_body
            }
        }
    };
    let extract_from_row = if base_struct.primary_key_field().is_some() {
        quote! {
            impl silo::ExtractFromRow for #name {
                fn try_from_row_simple(
                    column: &str,
                    row: &silo::rusqlite::Row,
                ) -> Result<Self, silo::Error> {unreachable!("should not be called directly!")}
                fn try_from_row(
                    column: &str,
                    row: &silo::rusqlite::Row,
                    connection: &silo::rusqlite::Connection,
                ) -> Result<Self, silo::Error> {
                    #extract_from_row_body
                }
            }
        }
    } else {
        quote! {}
    };
    tokens.extend(from_row);
    tokens.extend(extract_from_row);
}

fn create_extract_from_row_body(
    table_type: &Ident,
    base_struct: &crate::base_struct::StructData,
) -> proc_macro2::TokenStream {
    let Some(pk) = base_struct.primary_key_field() else {
        return quote! {};
    };
    let pk_type = pk.type_;
    let pk_name = pk.name;
    let filter = format_ident!("{}_equals", pk.name);
    quote! {
        use silo::SqlTable;
        let #pk_name: #pk_type = row.get(format!("{}_{}", column, stringify!(#pk_name)).as_str())?;
        let Some(#pk_name) = #pk_name else {
            return Ok(Default::default());
        };
        let __silo__db = unsafe { silo::Database::from_connection(connection)}?;
        let __silo__foreign = __silo__db.load::<#table_type>()?;
        let mut results = __silo__foreign.load_where(|f| f.#filter(#pk_name))?;
        assert_eq!(results.len(), 1, "Primary key was not unique!");
        Ok(results.pop().map(|r| r.into()).unwrap_or_default())
    }
}

fn create_try_from_row_body(
    base_struct: &crate::base_struct::StructData,
) -> proc_macro2::TokenStream {
    let columns = base_struct.columns();
    let column_names: Vec<syn::Ident> = columns.iter().map(|c| c.ident()).collect();
    let column_types = columns.iter().map(|c| c.type_);

    if let Some(variant) = base_struct.variant_field().map(|f| f.name) {
        quote! {todo!("Enums not yet supported!")}
    } else {
        quote! {#(
            let #column_names = <#column_types as silo::ExtractFromRow>::try_from_row(stringify!(#column_names), row, connection)?;
        )*
        Ok(Self {
            #(#column_names,)*
        })}
    }
}
