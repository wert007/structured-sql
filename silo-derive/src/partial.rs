use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::parse_quote;

use crate::type_checker::StripOption;

pub(crate) fn create_partial_for(
    base_struct: &crate::base_struct::StructData,
    strip_option_type_from_fields: bool,
    tokens: &mut proc_macro2::TokenStream,
) {
    let visibility = &base_struct.visibility;
    let name = &base_struct.name;
    let partial_name = base_struct.partial_name();
    let partial_type = create_partial_type_for(base_struct);
    // let variant_field = base_struct.variant_field().map(|f| f.name).into_iter();
    let field_names: Vec<_> = base_struct.fields().into_iter().map(|f| f.name).collect();
    let fields = base_struct
        .fields()
        .into_iter()
        .chain(base_struct.variant_field())
        .map(|f| {
            f.map_type(|t| {
                let t = if strip_option_type_from_fields {
                    t.strip_option()
                } else {
                    t
                };
                Box::leak(Box::new(parse_quote!(<#t as silo::HasPartial>::Partial)))
            })
        });

    let into = create_into_for(base_struct);
    tokens.extend(quote! {
        #[derive(Default)]
        #visibility struct #partial_name {
            #(#visibility #fields,)*
        }

        #partial_type

        impl silo::HasPartial for #name {
            type Partial = #partial_name;
        }

        #into

        impl silo::HasValue for #partial_name {
            fn has_values(&self) -> bool {
                #(self.#field_names.has_values() ||)* false
            }
        }

        impl silo::PartialRow for #partial_name {
            fn used_column_names(&self, column_name: Option<String>) -> Vec<String> {
                // TODO: Use with_capacity
                let mut result = Vec::new();
                #(
                    result.extend(self.#field_names.used_column_names(Some(column_name.clone().map(|c| format!("{c}_{}", stringify!(#field_names))).unwrap_or_else(|| stringify!(#field_names).to_string()))));
                )*
                result
            }
            fn used_values(&self) -> Vec<&dyn silo::rusqlite::ToSql> {
                    // TODO: Use with_capacity
                let mut result = Vec::new();
                #(
                    result.extend(self.#field_names.used_values());
                )*
                result

            }
        }
    });
}

fn create_into_for(base_struct: &crate::base_struct::StructData) -> TokenStream {
    let name = &base_struct.name;
    let partial_name = base_struct.partial_name();
    let field_names: Vec<_> = base_struct.fields().into_iter().map(|f| f.name).collect();
    let field_names_prefixed_with_optional: Vec<_> = base_struct
        .fields()
        .into_iter()
        .map(|f| format_ident!("optional_{}", f.name))
        .collect();
    if let Some(variant) = base_struct.variant_field() {
        let variant_name = variant.name;
        let variant_pattern = base_struct.variant_patterns();
        let variants_fields = base_struct
            .variants_fields()
            .into_iter()
            .map(|f| f.into_iter().map(|f| f.name()).collect::<Vec<_>>())
            .collect::<Vec<_>>();
        let variants_fields_prefixed_with_optional = base_struct
            .variants_fields()
            .into_iter()
            .map(|f| {
                f.into_iter()
                    .map(|f| format_ident!("optional_{}", f.name()))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        quote! {
            impl Into<#partial_name> for #name {
                fn into(self) -> #partial_name {
                    use silo::EnumHelper;
                    #(
                        #[allow(non_snake_case)]
                        let mut #field_names_prefixed_with_optional = Default::default();)*
                    let __silo_variant = self.variant();
                    match self {
                        #(#variant_pattern => {
                            #( #variants_fields_prefixed_with_optional = #variants_fields.into();)*;
                        })*
                    }
                    #partial_name {
                        #variant_name: __silo_variant.into(),
                        #(#field_names: #field_names_prefixed_with_optional,)*
                    }
                }
            }
        }
    } else {
        quote! {
            impl Into<#partial_name> for #name {
                fn into(self) -> #partial_name {
                    #partial_name {
                        #(#field_names: self.#field_names.into(),)*
                    }
                }
            }
        }
    }
}

fn create_partial_type_for(
    base_struct: &crate::base_struct::StructData,
) -> proc_macro2::TokenStream {
    let name = &base_struct.name;
    let partial_name = base_struct.partial_name();
    if let Some(variant_field) = base_struct.variant_field().map(|f| f.name) {
        quote! {
            impl silo::PartialType<#name> for #partial_name {
                fn transpose(self) -> Option<#name> {
                    use silo::PartialType;
                    let #variant_field = self.#variant_field.transpose()?;
                    match #variant_field {
                        _ => None
                    }
                }
            }
        }
    } else {
        let field_names: Vec<_> = base_struct.fields().into_iter().map(|f| f.name).collect();
        let skipped_field_names = base_struct.skipped_fields().into_iter().map(|f| f.name);
        quote! {
            impl silo::PartialType<#name> for #partial_name {
                fn transpose(self) -> Option<#name> {
                    use silo::PartialType;
                    #(let #field_names = self.#field_names.transpose()?;)*
                    Some(#name {
                        #(#field_names,)*
                        #(#skipped_field_names: Default::default(),)*
                    })
                }
            }
        }
    }
}
