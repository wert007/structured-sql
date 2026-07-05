use itertools::Itertools;
use quote::quote;
use syn::{LitStr, ext::IdentExt};

pub(crate) fn impl_extract_from_row(
    tokens: &mut proc_macro2::TokenStream,
    base_struct: &crate::base_struct::StructData,
) {
    if base_struct.is_enum() {
        if base_struct.is_simple_enum() {
            impl_extract_from_row_for_simple_enum(tokens, base_struct);
        } else {
            tokens.extend(quote! {compile_error!("No enums with fields currently supported.")});
        }
    } else {
        impl_extract_from_row_struct(tokens, base_struct);
    }
}

fn impl_extract_from_row_for_simple_enum(
    tokens: &mut proc_macro2::TokenStream,
    base_struct: &crate::base_struct::StructData,
) {
    let name = &base_struct.name;
    let variants = base_struct.variant_patterns();
    let discriminant_values = base_struct.discriminant_values();
    tokens.extend(quote! {
        impl silo::ExtractFromRow for #name {
            fn try_from_row_simple(column_name: &str, row: &silo::rusqlite::Row) -> Result<Self, silo::Error> {
                match row.get::<&str, String>(column_name) {
                    Ok(it) => match it.as_str() {
                        #(#discriminant_values => Ok(#variants),)*
                        err => Err(silo::Error::UnknownEnumVariant(err.to_string(), stringify!(#name))),
                    },
                    Err(silo::rusqlite::Error::InvalidColumnName(_)) => {
                        Err(silo::Error::MissingColumn(column_name.to_string().into()))
                    }
                    Err(silo::rusqlite::Error::InvalidColumnType(.., t)) => {
                        Err(silo::Error::WrongColumnType(stringify!(#name).into(), t))
                    }
                    Err(err) => unreachable!("Impossible error? {err}"),
                }
            }
        }
    });
}

fn impl_extract_from_row_struct(
    tokens: &mut proc_macro2::TokenStream,
    base_struct: &crate::base_struct::StructData,
) {
    let name = &base_struct.name;
    let fields = base_struct.fields();
    let field_names = fields.iter().map(|f| f.name).collect_vec();
    let field_names_literals = fields.iter().map(|f| {
        let n = f.name.unraw();
        LitStr::new(&n.to_string(), n.span())
    });
    let field_types = fields.iter().map(|f| f.type_).collect_vec();
    tokens.extend(quote! {
        impl silo::ExtractFromRow for #name {
            fn try_from_row_simple(column_name: &str, row: &silo::rusqlite::Row) -> std::result::Result<Self, silo::Error> {
                let mut result = std::mem::MaybeUninit::uninit();
                let ptr: *mut #name = result.as_mut_ptr();
                #(
                    unsafe {
                        (&raw mut (*ptr).#field_names).write(<#field_types>::try_from_row_simple(&[column_name, concat!("_", #field_names_literals)].concat(), row)?);
                    }
                )*
                Ok(unsafe {
                    result.assume_init()
                })
            }

        }
    });
}
