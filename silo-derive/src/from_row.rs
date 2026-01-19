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
    let body = if base_struct.is_partial {
        create_try_from_row_body(base_struct)
    } else {
        let partial = base_struct.partial_name();
        quote!(
            use silo::PartialType;
            #partial::try_from_row(string_storage, column_name, row, connection)?.transpose())
    };
    let iter = quote! {
        impl silo::FromRow for #name {
            fn try_from_row(
                string_storage: &mut silo::StaticStringStorage,
                column_name: Option<std::borrow::Cow<'static, str>>,
                row: &silo::rusqlite::Row,
                connection: &silo::rusqlite::Connection,
            ) -> Option<Self> {
                #body
            }
        }
    };
    tokens.extend(iter);
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
            let #column_names = <#column_types as silo::FromRow>::try_from_row(string_storage, Some(column_name.clone().map(|c| [&c, "_", stringify!(#column_names)].join("").into()).unwrap_or(stringify!(#column_names).into())), row, connection)?;
        )*
        Some(Self {
            #(#column_names,)*
        })}
    }
}
