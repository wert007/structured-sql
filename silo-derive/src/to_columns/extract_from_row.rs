use crate::to_table;
use itertools::Itertools;
use quote::quote;

pub(crate) fn impl_extract_from_row(
    tokens: &mut proc_macro2::TokenStream,
    base_struct: &crate::base_struct::StructData,
) {
    let name = &base_struct.name;
    let fields = base_struct.fields();
    let field_names = fields.iter().map(|f| f.name).collect_vec();
    let field_types = fields.iter().map(|f| f.type_).collect_vec();
    tokens.extend(quote! {
        impl silo::ExtractFromRow for #name {
            fn try_from_row_simple(column_name: &str, row: &silo::rusqlite::Row) -> std::result::Result<Self, silo::Error> {
                let mut result = std::mem::MaybeUninit::uninit();
                let ptr: *mut #name = result.as_mut_ptr();
                #(
                    unsafe {
                        (&raw mut (*ptr).#field_names).write(<#field_types>::try_from_row_simple(&format!("{column_name}_{}", stringify!(#field_names)), row)?);
                    }
                )*
                Ok(unsafe {
                    result.assume_init()
                })
            }

        }
    });
}
