use quote::quote;

pub(crate) fn create_as_params(
    base_struct: &crate::base_struct::StructData,
    tokens: &mut proc_macro2::TokenStream,
) {
    let name = &base_struct.name;
    let columns = base_struct.columns();
    let column_types = columns.iter().map(|c| &c.type_);
    let names = columns.iter().map(|c| syn::Ident::new(&c.name, c.span));
    let as_params = if let Some(variant) = base_struct.variant_field() {
        let partial_name = base_struct.partial_name();
        let variant_types = base_struct
            .variants_fields()
            .into_iter()
            .map(|v| v.into_iter().map(|f| f.type_).collect::<Vec<_>>())
            .collect::<Vec<_>>();
        let mut previous_types = Vec::new();
        for (i, t) in variant_types.iter().enumerate() {
            let mut types = Vec::new();
            for i in 0..i {
                types.extend(variant_types[i].clone());
            }
            previous_types.push(types);
        }
        let variants_pattern = base_struct.variant_patterns();
        let variants_fields = base_struct
            .variants_fields()
            .into_iter()
            .map(|v| v.into_iter().map(|v| v.name()).collect::<Vec<_>>())
            .collect::<Vec<_>>();
        quote! {
            impl silo::AsParams for #name {
                fn as_params<'a>(&'a self) -> Vec<&'a dyn silo::rusqlite::ToSql> {
                    use silo::{HasColumnCount, EnumHelper};
                    let mut result: Vec<&'a dyn silo::rusqlite::ToSql> = Vec::with_capacity(Self::COLUMN_COUNT);
                    result.push(self.variant_ref());
                    match self {
                        #(#variants_pattern => {
                            for _ in 0..(0#(+ <#previous_types as silo::HasColumnCount>::COLUMN_COUNT)*) {
                                result.push(&silo::rusqlite::types::Null);
                            }
                            #(result.extend(#variants_fields.as_params());)*
                        })*
                    }
                    while result.len() < Self::COLUMN_COUNT {
                        result.push(&silo::rusqlite::types::Null);
                    }
                    // #(if let Some(value) = partial.#names {
                    // } else {
                    //     result.extend([&silo::rus]);

                    // })*
                    // #(
                    //     result.extend(self.#names.as_params());
                    // )*
                    result
                }
            }
        }
    } else {
        quote! {
            impl silo::AsParams for #name {
                fn as_params<'a>(&'a self) -> Vec<&'a dyn silo::rusqlite::ToSql> {
                    use silo::HasColumnCount;
                    let mut result = Vec::with_capacity(Self::COLUMN_COUNT);
                    #(
                        result.extend(self.#names.as_params());
                    )*
                    result
                }
            }
        }
    };
    let iter = quote! {
        impl silo::HasColumnCount for #name {
            const COLUMN_COUNT: usize = 0 #(+ <#column_types as silo::HasColumnCount>::COLUMN_COUNT)*;
        }

       #as_params
    };
    tokens.extend(iter);
}
