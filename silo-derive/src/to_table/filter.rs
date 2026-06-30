use itertools::Itertools;
use quote::quote;
use syn::{LitStr, ext::IdentExt};

pub(crate) fn create_filter_for(
    base_struct: &super::base_struct::StructData,
) -> proc_macro2::TokenStream {
    let visibility = &base_struct.visibility;
    let filter_name = base_struct.filter_name();
    let name = &base_struct.name;

    let fields = base_struct
        .fields()
        .into_iter()
        .map(|f| f.name)
        .collect_vec();
    let fields_str_lit = fields.iter().map(|f| {
        let n = f.unraw();
        LitStr::new(&n.to_string(), n.span())
    });
    let field_types = base_struct.fields().into_iter().map(|f| f.type_);
    let from_pk = if let Some(pk) = base_struct.primary_key_field() {
        let pk_type = pk.type_;
        let pk_ident = pk.name;
        quote! {
            impl From<#pk_type> for #filter_name {
                fn from(#pk_ident: #pk_type) -> Self {
                    use silo::filter::Filterable;
                    Self {
                        #pk_ident: #pk_ident.convert_to_equals_filter(),
                        ..Default::default()
                    }
                }
            }
        }
    } else {
        quote! {}
    };
    quote! {
        #[derive(Default)]
        #visibility struct #filter_name {
            #(pub #fields: <#field_types as silo::filter::Filterable>::Filter,)*
        }

        #from_pk

        impl From<()> for #filter_name {
            fn from((): ()) -> Self {
                Self::default()
            }
        }

        impl silo::filter::Filter for #filter_name {
            fn to_sql(&self, sql: &mut String, parent: Option<&str>) {
                let parent = parent.map(|p| format!("{p}_")).unwrap_or_default();
                #(
                    self.#fields.to_sql(sql, Some(&format!("{parent}{}", #fields_str_lit)));
                )*
            }
        }

        impl silo::AsParams for #filter_name {
            fn as_params<'a>(&'a self) -> Vec<silo::ToSqlDyn<'a>> {
                    use silo::{AsParams};
                    let mut result = Vec::new();
                    #(
                        result.extend(AsParams::as_params(&self.#fields));
                    )*
                    result
                }
        }

        impl silo::filter::Filterable for #name {
            type Filter = #filter_name;
            fn convert_to_equals_filter(self) -> Self::Filter {
                Self::Filter {
                    #(
                        #fields: self.#fields.convert_to_equals_filter(),
                    )*
                }
            }
        }
    }
}
