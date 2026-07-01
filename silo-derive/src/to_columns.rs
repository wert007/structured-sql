use crate::base_struct;
use quote::ToTokens;
use syn::{Ident, Visibility};

mod as_params;
mod extract_from_row;
mod filterable;
mod partial;

pub struct ToColumnsStruct {
    visibility: Visibility,
    base_struct: base_struct::StructData,
}

impl ToColumnsStruct {
    pub fn from_struct(
        _attrs: Vec<syn::Attribute>,
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

impl ToTokens for ToColumnsStruct {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        partial::impl_to_partial(tokens, &self.base_struct);
        filterable::impl_filterable(tokens, &self.base_struct);
        extract_from_row::impl_extract_from_row(tokens, &self.base_struct);
        as_params::impl_as_params(tokens, &self.base_struct);
    }
}
