use quote::quote;

pub(crate) fn create_as_params(
    base_struct: &crate::base_struct::StructData,
    tokens: &mut proc_macro2::TokenStream,
    for_table: bool,
) {
    let name = &base_struct.name;
    let columns = base_struct.columns();
    let column_types = columns.iter().map(|c| &c.type_);
    let names = columns.iter().map(|c| syn::Ident::new(&c.name, c.span));
    let _trait = if for_table {
        quote! {silo::TableAsParams}
    } else {
        quote! {silo::AsParams}
    };
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
            impl #_trait for #name {
                fn as_params<'a>(&'a self) -> Vec<&'a dyn silo::rusqlite::ToSql> {
                    use silo::{EnumHelper, AsParams};
                    let mut result: Vec<&'a dyn silo::rusqlite::ToSql> = Vec::with_capacity(Self::COLUMN_COUNT);
                    result.push(self.variant_ref());
                    match self {
                        #(#variants_pattern => {
                            for _ in 0..(0#(+ <#previous_types>::COLUMN_COUNT)*) {
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
            impl #_trait for #name {
                const COLUMN_COUNT: usize = 0 #(+ <#column_types as silo::AsParams>::COLUMN_COUNT)*;
                fn as_params<'a>(&'a self) -> Vec<&'a dyn silo::rusqlite::ToSql> {
                    use silo::{AsParams};
                    let mut result = Vec::with_capacity(<Self as #_trait>::COLUMN_COUNT);
                    #(
                        result.extend(AsParams::as_params(&self.#names));
                    )*
                    result
                }
            }
        }
    };
    tokens.extend(as_params);
}

pub(crate) fn create_as_params_for_pk(
    base_struct: &crate::base_struct::StructData,
    tokens: &mut proc_macro2::TokenStream,
) {
    let Some(pk) = base_struct.primary_key_field() else {
        return;
    };
    let pk_name = pk.name;
    let name = &base_struct.name;
    let as_params = quote! {
        impl silo::AsForeignReference for #name {
            fn insert_as_foreign_reference(
                self,
                connection: &rusqlite::Connection,
            ) -> Result<(), rusqlite::Error> {
                use silo::SqlTable;
                let db = unsafe {silo::Database::from_connection(connection)}?;
                let table = db.load::<Self>()?;
                table.insert(self)?;
                Ok(())
            }
        }

        impl silo::AsParams for #name {
            const COLUMN_COUNT: usize = 1;
            fn as_params<'a>(&'a self) -> Vec<&'a dyn silo::rusqlite::ToSql> {
                let mut result = Vec::with_capacity(Self::COLUMN_COUNT);
                result.extend(self.#pk_name.as_params());
                result
            }
        }
    };
    tokens.extend(as_params);
}
