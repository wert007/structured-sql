use crate::{attributes, base_struct};
use quote::ToTokens;
use syn::{Ident, Visibility};

mod as_params;
mod extract_from_row;
mod filterable;
mod partial;

pub struct ToColumns {
    visibility: Visibility,
    base_struct: base_struct::StructData,
}

impl ToColumns {
    pub fn from_struct(
        attrs: Vec<syn::Attribute>,
        name: Ident,
        visibility: Visibility,
        data_struct: syn::DataStruct,
    ) -> Result<Self, crate::error::Error> {
        // let attribute_struct_data = attributes::ToTableAttributesStruct::parse(&attrs);
        // let on_conflict = attribute_struct_data.on_conflict();

        let base_struct: base_struct::StructData = base_struct::StructData::from_struct_data(
            visibility.clone(),
            name.clone(),
            data_struct.fields,
        )?;
        Ok(Self {
            visibility,
            base_struct,
        })
    }
}

impl ToTokens for ToColumns {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        partial::impl_to_partial(tokens, &self.base_struct);
        filterable::impl_filterable(tokens, &self.base_struct);
        extract_from_row::impl_extract_from_row(tokens, &self.base_struct);
        as_params::impl_as_params(tokens, &self.base_struct);
    }
}
