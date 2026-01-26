use quote::quote;

pub(crate) fn create_from_row_type(
    base_struct: &crate::base_struct::StructData,
    row_type: &crate::base_struct::StructData,
    tokens: &mut proc_macro2::TokenStream,
) {
    let primary_key = base_struct
        .primary_key_field()
        .expect("Should have checked before!")
        .name;
    let non_vec_fields = base_struct
        .non_vec_fields()
        .into_iter()
        .map(|f| f.name)
        .collect::<Vec<_>>();
    let row_type_name = &row_type.name;
    let name = &base_struct.name;
    tokens.extend(quote! {
        impl silo::FromRowType<#row_type_name> for #name {
            fn from_row_type(value: Vec<#row_type_name>) -> Vec<Self> {
                if value.is_empty() {
                    return Vec::new();
                }
                let mut result = Vec::new();
                #(let mut #non_vec_fields = value[0].#non_vec_fields.expect("First value should always be set!");)*
                for value in value {
                    if value.#primary_key != #primary_key {
                        result.push(Self {
                            #(#non_vec_fields,)*
                        });
                        #(#non_vec_fields = value.#non_vec_fields.expect("First value should always be set!");)*
                    }
                }
                result
            }

        }
    });
}
