use itertools::Itertools;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{LitStr, ext::IdentExt, parse_quote};

pub(crate) fn create_partial_for(
    base_struct: &super::base_struct::StructData,
    tokens: &mut proc_macro2::TokenStream,
) {
    if base_struct.is_simple_enum() {
        return;
    }
    let visibility = &base_struct.visibility;
    let name = &base_struct.name;
    let where_clause = &base_struct.where_clause;
    let generics = &base_struct.generics;
    let generics_names_only = base_struct.generics_names_only(TokenStream::new());
    let partial_name = base_struct.partial_name();
    let partial_type = create_partial_type_for(base_struct);
    // let variant_field = base_struct.variant_field().map(|f| f.name).into_iter();
    let field_names: Vec<_> = base_struct.fields().into_iter().map(|f| f.name).collect();
    let field_names_str_lit = field_names.iter().map(|f| {
        let n = f.unraw();
        LitStr::new(&n.to_string(), n.span())
    });
    let is_unique = base_struct
        .columns()
        .into_iter()
        .map(|c| syn::LitBool::new(c.is_unique, c.span));
    let is_primary = base_struct
        .columns()
        .into_iter()
        .map(|c| syn::LitBool::new(c.is_primary, c.span));
    let fields = base_struct
        .fields()
        .into_iter()
        .chain(base_struct.variant_field())
        .map(|f| {
            f.map_type(|t| {
                Box::leak(Box::new(
                    parse_quote!(<#t as silo::partial::HasPartial>::Partial),
                ))
            })
        })
        .collect_vec();

    let into = create_into_for(base_struct);
    tokens.extend(quote! {
        #visibility struct #partial_name #generics #where_clause {
            #(#visibility #fields,)*
        }

        impl #generics Default for #partial_name #generics_names_only #where_clause {
            fn default() -> Self {
                Self {
                    #(
                        #field_names: Default::default(),
                    )*
                }
            }
        }

        #partial_type

        impl #generics silo::partial::HasPartial for #name #generics_names_only #where_clause {
            type Partial = #partial_name #generics_names_only;
        }

        impl #generics silo::AsColumnsOptional for #partial_name #generics_names_only #where_clause {
            fn columns_skip_optional(
        &self,
        parent: Option<&str>,
        is_unique: bool,
        is_primary: bool,
    ) -> Vec<silo::SqlColumn> {
        let parent = parent.map(|p| format!("{p}_")).unwrap_or_default();
                let mut result = Vec::new();
                #(result.append(&mut self.#field_names.columns_skip_optional(Some(&format!("{parent}{}", #field_names_str_lit)), #is_unique, #is_primary));)*
                result
    }
        }

        impl #generics silo::AsParamsOptional for #partial_name #generics_names_only #where_clause {
            fn as_params_skip_optional<'b>(&'b self) -> Vec<silo::ToSqlDyn<'b>> {
                let mut result = Vec::new();
                #(result.append(&mut self.#field_names.as_params_skip_optional());)*
                result
            }
        }

        #into

        // impl silo::HasValue for #partial_name {
        //     fn has_values(&self) -> bool {
        //         #(self.#field_names.has_values() ||)* false
        //     }
        // }

        // impl silo::PartialRow for #partial_name {
        //     fn used_column_names(&self, column_name: Option<String>) -> Vec<String> {
        //         // TODO: Use with_capacity
        //         let mut result = Vec::new();
        //         #(
        //             result.extend(self.#field_names.used_column_names(Some(column_name.clone().map(|c| format!("{c}_{}", #field_names_str_lit)).unwrap_or_else(|| #field_names_str_lit.to_string()))));
        //         )*
        //         result
        //     }
        //     fn used_values(&self) -> Vec<&dyn silo::rusqlite::ToSql> {
        //             // TODO: Use with_capacity
        //         let mut result = Vec::new();
        //         #(
        //             result.extend(self.#field_names.used_values());
        //         )*
        //         result

        //     }
        // }
    });
}

fn create_into_for(base_struct: &super::base_struct::StructData) -> TokenStream {
    let name = &base_struct.name;
    let where_clause = &base_struct.where_clause;
    let generics = &base_struct.generics;
    let generics_names_only = base_struct.generics_names_only(TokenStream::new());
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
            impl #generics Into<#partial_name #generics_names_only> for #name #generics_names_only #where_clause {
                fn into(self) -> #partial_name #generics_names_only {
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
            impl #generics Into<#partial_name #generics_names_only> for #name #generics_names_only #where_clause {
                fn into(self) -> #partial_name #generics_names_only {
                    #partial_name {
                        #(#field_names: self.#field_names.into(),)*
                    }
                }
            }
        }
    }
}

fn create_partial_type_for(
    base_struct: &super::base_struct::StructData,
) -> proc_macro2::TokenStream {
    let name = &base_struct.name;
    let generics = &base_struct.generics;
    let where_clause = &base_struct.where_clause;
    let generics_names_only = &base_struct.generics_names_only(TokenStream::new());
    let partial_name = base_struct.partial_name();
    if let Some(variant_field) = base_struct.variant_field().map(|f| f.name) {
        quote! {
            impl #generics silo::partial::PartialType<#name #generics_names_only> for #partial_name #generics_names_only #where_clause {
                fn transpose(self) -> Option<#name #generics_names_only> {
                    use silo::partial::PartialType;
                    let #variant_field = self.#variant_field.transpose()?;
                    match #variant_field {
                        _ => None
                    }
                }
            }
        }
    } else {
        let field_names: Vec<_> = base_struct.fields().into_iter().map(|f| f.name).collect();
        let skipped_fields = base_struct.skipped_fields().into_iter().map(|f| {
            let initializer = f
                .default
                .cloned()
                .unwrap_or_else(|| parse_quote!(Default::default()));
            let n = f.name;
            quote! {
                #n: #initializer,
            }
        });
        quote! {
            impl #generics silo::partial::PartialType<#name #generics_names_only> for #partial_name #generics_names_only #where_clause {
                fn transpose(self) -> Option<#name #generics_names_only> {
                    use silo::partial::PartialType;
                    #(let #field_names = self.#field_names.transpose()?;)*
                    Some(#name {
                        #(#field_names,)*
                        #(#skipped_fields)*
                    })
                }
            }
        }
    }
}
