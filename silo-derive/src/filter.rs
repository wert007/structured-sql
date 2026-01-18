use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::parse_quote;

pub(crate) fn create_filter_for(
    base_struct: &crate::base_struct::StructData,
) -> proc_macro2::TokenStream {
    let visibility = &base_struct.visibility;
    let name = &base_struct.name;
    let filter_name = base_struct.filter_name();
    let field_names: Vec<_> = base_struct
        .fields()
        .into_iter()
        .chain(base_struct.variant_field())
        .map(|f| f.name)
        .collect();
    let fields = base_struct
        .fields()
        .into_iter()
        .chain(base_struct.variant_field())
        .map(|f| {
            f.map_type(|t| Box::leak(Box::new(parse_quote!(<#t as silo::HasFilter>::Filter))))
        });
    let has_filter = create_has_filter_for(base_struct);
    quote! {
        #[derive(Default)]
        #visibility struct #filter_name {
            #(#visibility #fields,)*
        }

        #has_filter

        impl silo::IntoGenericFilter for #filter_name {
            fn into_generic(
                self,
                string_storage: &mut silo::StaticStringStorage,
                column_name: Option<std::borrow::Cow<'static, str>>,
            ) -> silo::GenericFilter {
                let mut result = silo::GenericFilter::default();
                #(
                    result.insert(stringify!(#field_names).into(), self.#field_names, string_storage);
                )*
                result
            }
        }
    }
}

fn create_has_filter_for(base_struct: &crate::base_struct::StructData) -> TokenStream {
    let name = &base_struct.name;
    let filter_name = base_struct.filter_name();
    let field_names: Vec<_> = base_struct.fields().into_iter().map(|f| f.name).collect();

    if let Some(variant) = base_struct.variant_field() {
        let variant = variant.name;
        let variant_patterns = base_struct.variant_patterns();
        let variant_fields = base_struct
            .variants_fields()
            .into_iter()
            .map(|v| v.into_iter().map(|v| v.name()).collect::<Vec<_>>())
            .collect::<Vec<_>>();
        quote! {
            impl silo::HasFilter for #name {
                type Filter = #filter_name;

                fn must_be_equal(&self) -> Self::Filter {
                    use silo::EnumHelper;
                    let mut result = #filter_name::default();
                    result.#variant = self.variant();
                    match self {
                        #(#variant_patterns => {
                            #(result.#variant_fields = #variant_fields.must_be_equal();)*
                        })*
                    }
                    result
                }

                fn must_contain(&self) -> Self::Filter {
                    use silo::EnumHelper;
                    let mut result = #filter_name::default();
                    result.#variant = self.variant();
                    match self {
                        #(#variant_patterns => {
                            #(result.#variant_fields = #variant_fields.must_contain();)*
                        })*
                    }
                    result
                }
            }
        }
    } else {
        quote! {
            impl silo::HasFilter for #name {
                type Filter = #filter_name;

                fn must_be_equal(&self) -> Self::Filter {
                    #filter_name {
                        #(#field_names: self.#field_names.must_be_equal(),)*
                    }
                }

                fn must_contain(&self) -> Self::Filter {
                    #filter_name {
                        #(#field_names: self.#field_names.must_contain(),)*
                    }
                }
            }
        }
    }
}
