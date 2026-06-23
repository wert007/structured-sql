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
            use silo::partial::PartialType;
            #partial::try_from_row(row, connection)?.transpose().ok_or(silo::Error::Todo("Improve error handling here, so we know which column was missing".into()))
        )
    };
    let from_row = quote! {
        impl silo::FromRow for #name {
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
