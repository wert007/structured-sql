use quote::quote;

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
            #partial::try_from_row(row, connection)?.transpose())
    };
    let extract_from_row_body = if base_struct.is_partial {
        create_extract_from_row_body(base_struct)
    } else {
        let partial = base_struct.partial_name();
        quote!(
            use silo::PartialType;
            #partial::try_from_row(column, row, connection)?.transpose()
        )
    };
    let from_row = quote! {
        impl silo::FromRow for #name {
            fn try_from_row(
                row: &silo::rusqlite::Row,
                connection: &silo::rusqlite::Connection,
            ) -> Option<Self> {
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
                ) -> Option<Self> {None}
                fn try_from_row(
                    column: &str,
                    row: &silo::rusqlite::Row,
                    connection: &silo::rusqlite::Connection,
                ) -> Option<Self> {
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
    base_struct: &crate::base_struct::StructData,
) -> proc_macro2::TokenStream {
    let unit = syn::parse_quote!(());
    let pk_type = base_struct
        .primary_key_field()
        .map(|f| f.type_)
        .unwrap_or(&unit);
    quote! {
        let pk: #pk_type = row.get(column).ok()?;
        todo!("Support loading foreign keys. (Needs filtering back!)")
    }
}

fn create_try_from_row_body(
    base_struct: &crate::base_struct::StructData,
) -> proc_macro2::TokenStream {
    let columns = base_struct.columns();
    let column_names: Vec<syn::Ident> = columns.iter().map(|c| c.ident()).collect();
    let column_types = columns.iter().map(|c| c.type_);

    if let Some(variant) = base_struct.variant_field().map(|f| f.name) {
        quote! {None}
    } else {
        quote! {#(
            let #column_names = <#column_types as silo::ExtractFromRow>::try_from_row(stringify!(#column_names), row, connection)?;
        )*
        Some(Self {
            #(#column_names,)*
        })}
    }
}
