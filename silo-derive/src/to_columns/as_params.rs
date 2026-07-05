use quote::quote;

use crate::to_table;

pub(crate) fn impl_as_params(
    tokens: &mut proc_macro2::TokenStream,
    base_struct: &crate::base_struct::StructData,
) {
    if base_struct.is_enum() {
        if base_struct.is_simple_enum() {
            create_as_params_for_simple_enum(tokens, base_struct);
        } else {
            tokens.extend(quote! {compile_error!("Currently no fields in enums are supported.")});
        }
    } else {
        to_table::as_params::create_as_params_for_struct(base_struct, tokens, false);
    }
}

fn create_as_params_for_simple_enum(
    tokens: &mut proc_macro2::TokenStream,
    base_struct: &crate::base_struct::StructData,
) {
    let name = &base_struct.name;
    let variants = base_struct.variant_patterns();
    let discriminant_values = base_struct.discriminant_values();
    tokens.extend(quote! {
        impl silo::IsSingleColumn for #name {
            const SQL_COLUMN_TYPE: silo::SqlColumnType = silo::SqlColumnType::Text;
        }

        impl silo::AsParams for #name {
            fn as_params<'b>(&'b self) -> Vec<silo::ToSqlDyn<'b>> {
                vec![silo::ToSqlDyn::Borrowed(match self {
                    #(#variants => &#discriminant_values,)*
                })]
            }
        }
    });
}
