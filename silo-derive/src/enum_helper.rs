use quote::quote;

pub(crate) fn create_enum_helper_for(
    base_struct: &crate::base_struct::StructData,
    tokens: &mut proc_macro2::TokenStream,
) {
    let name = &base_struct.name;
    let variant_patterns = base_struct.variant_patterns();
    let variants = base_struct.variant_names();
    let iter = quote! {
        impl silo::EnumHelper for #name {
            #[allow(unused_variables)]
            fn variant_ref(&self) -> &'static &'static str {
                match self {
                    #(#variant_patterns => &stringify!(#variants),)*
                }
            }
        }
    };
    tokens.extend(iter);
}
