use quote::quote;

use crate::to_table;

pub(crate) fn impl_filterable(
    tokens: &mut proc_macro2::TokenStream,
    base_struct: &crate::base_struct::StructData,
) {
    if base_struct.is_simple_enum() {
        impl_filterable_for_simple_enum(tokens, base_struct);
    } else {
        tokens.extend(to_table::filter::create_filter_for(base_struct));
    }
}

fn impl_filterable_for_simple_enum(
    tokens: &mut proc_macro2::TokenStream,
    base_struct: &crate::base_struct::StructData,
) {
    let name = &base_struct.name;
    let variants = base_struct.variant_patterns();
    let discriminant_values = base_struct.discriminant_values();
    tokens.extend(quote! {

        impl sqilo::filter::Filterable for #name {
            type Filter = sqilo::filter::FieldFilter<String>;

            fn convert_to_equals_filter(self) -> Self::Filter {
                match self {
                    #(#variants => #discriminant_values.to_string().convert_to_equals_filter(),)*
                }
            }
        }

    });
}
