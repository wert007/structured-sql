use crate::{attributes::ToColumnsAttributesEnum, base_struct};
use quote::ToTokens;
use syn::{Generics, Ident, Visibility};

mod as_params;
mod extract_from_row;
mod filterable;
mod marker_trait;
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
        generics: Generics,
        data_struct: syn::DataStruct,
    ) -> Result<Self, crate::error::Error> {
        // let attribute_struct_data = attributes::ToTableAttributesStruct::parse(&attrs);
        // let on_conflict = attribute_struct_data.on_conflict();

        let base_struct: base_struct::StructData = base_struct::StructData::from_struct_data(
            visibility.clone(),
            name.clone(),
            generics,
            data_struct.fields,
        )?;
        Ok(Self {
            visibility,
            base_struct,
        })
    }

    pub(crate) fn from_enum(
        attrs: Vec<syn::Attribute>,
        name: Ident,
        visibility: Visibility,
        generics: Generics,
        data_enum: syn::DataEnum,
    ) -> Result<ToColumnsStruct, crate::error::Error> {
        let attr = ToColumnsAttributesEnum::parse(&attrs)?;
        let base_struct: base_struct::StructData = base_struct::StructData::from_enum_data(
            visibility.clone(),
            name.clone(),
            generics,
            data_enum.variants,
            attr,
        )?;
        Ok(Self {
            visibility,
            base_struct,
        })
    }
}

impl ToTokens for ToColumnsStruct {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        marker_trait::impl_marker_trait(tokens, &self.base_struct);
        partial::impl_to_partial(tokens, &self.base_struct);
        filterable::impl_filterable(tokens, &self.base_struct);
        extract_from_row::impl_extract_from_row(tokens, &self.base_struct);
        as_params::impl_as_params(tokens, &self.base_struct);
    }
}
