use crate::attributes::AttributeFieldData;
use crate::attributes::ToColumnsAttributesEnum;
use crate::error::Error;
use convert_case::{Case, Casing};
use itertools::Itertools;
use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, format_ident, quote};
use syn::Generics;
use syn::WhereClause;
use syn::ext::IdentExt;
use syn::{Ident, Type, Visibility, parse_quote, spanned::Spanned};

#[derive(Clone, Copy)]
pub struct Field<'a> {
    pub name: &'a Ident,
    pub type_: &'a Type,
    pub default: Option<&'a syn::Expr>,
}
impl Field<'_> {
    pub(crate) fn map_type(self, f: impl Fn(&Type) -> &Type) -> Self {
        Self {
            name: self.name,
            type_: f(self.type_),
            default: self.default,
        }
    }
}

impl ToTokens for Field<'_> {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let name = self.name;
        let type_ = self.type_;
        tokens.extend(quote! {#name: #type_});
    }
}

#[derive(Clone)]
pub struct Member {
    variant: Option<Ident>,
    name: Ident,
    visibility: Visibility,
    type_: Type,
    is_primary: bool,
    is_unique: bool,
    is_skipped: bool,
    is_remaining_element: bool,
    is_unnamed: bool,
    default: Option<syn::Expr>,
}

impl std::fmt::Debug for Member {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Member")
            .field("variant", &self.variant)
            .field("name", &self.name)
            .field("is_primary", &self.is_primary)
            .field("is_unique", &self.is_unique)
            .field("is_skipped", &self.is_skipped)
            .field("is_unnamed", &self.is_unnamed)
            .finish()
    }
}
impl Member {
    fn to_partial(self) -> Self {
        let t = self.type_;
        Member {
            variant: self.variant,
            name: self.name,
            visibility: self.visibility,
            type_: syn::parse_quote!(<#t as silo::partial::HasPartial>::Partial),
            is_primary: self.is_primary,
            is_unique: self.is_unique,
            is_skipped: self.is_skipped,
            is_remaining_element: self.is_remaining_element,
            is_unnamed: self.is_unnamed,
            default: self.default.map(|d| parse_quote! { Some(#d) }),
        }
    }

    fn generate_field_name(index: usize, f: &syn::Field) -> (Ident, bool) {
        let mut ident = format_ident!("_{index}");
        ident.set_span(f.span());
        (ident, true)
    }
    fn from_field_with_attributes(index: usize, a: &AttributeFieldData, f: &syn::Field) -> Member {
        let (name, name_is_generated) = f
            .ident
            .clone()
            .map(|i| (i, false))
            .unwrap_or_else(|| Self::generate_field_name(index, f));
        Self {
            variant: None,
            name,
            visibility: f.vis.clone(),
            type_: f.ty.clone(),
            is_primary: a.is_primary,
            is_unique: a.is_unique,
            is_skipped: a.is_skip,
            is_remaining_element: false,
            is_unnamed: name_is_generated,
            default: a.default.clone(),
        }
    }

    fn create_variant_member(span: proc_macro2::Span) -> Member {
        let name = syn::Ident::new("__silo_variant", span);
        let visibility = syn::parse_quote!(pub);
        let type_ = syn::parse_quote!(&'static str);
        Member {
            variant: None,
            name,
            visibility,
            type_,
            is_primary: false,
            is_unique: false,
            is_skipped: false,
            is_remaining_element: false,
            is_unnamed: false,
            default: None,
        }
    }

    fn to_field(&self) -> Field<'_> {
        Field {
            name: &self.name,
            type_: &self.type_,
            default: self.default.as_ref(),
        }
    }

    fn to_column_data(&self) -> ColumnData<'_> {
        ColumnData {
            span: self.name.span(),
            name: self.name.to_string(),
            type_: &self.type_,
            is_unique: self.is_unique,
            is_primary: self.is_primary,
            default_value: self.default.clone(),
        }
    }
}

#[derive(Clone)]
pub struct ColumnData<'a> {
    pub span: proc_macro2::Span,
    pub name: String,
    pub type_: &'a Type,
    pub is_unique: bool,
    pub is_primary: bool,
    pub default_value: Option<syn::Expr>,
}
impl ColumnData<'_> {
    pub(crate) fn ident(&self) -> syn::Ident {
        format_ident!("{}", &self.name, span = self.span)
    }
}

#[derive(Clone)]
pub struct VariantField {
    pub index: usize,
    pub name: Option<Ident>,
    pub type_: Type,
    pub span: Span,
}

impl VariantField {
    pub fn name(&self) -> Ident {
        self.name.clone().unwrap_or_else(|| {
            let mut ident = format_ident!("_{}", self.index);
            ident.set_span(self.span);
            ident
        })
    }
}

#[derive(Clone)]
pub struct VariantData {
    type_name: Ident,
    name: Ident,
    fields: Vec<VariantField>,
}

impl VariantData {
    fn new(type_name: Ident, v: &syn::Variant, index_offset: usize) -> Self {
        Self {
            type_name,
            name: v.ident.clone(),
            fields: v
                .fields
                .iter()
                .enumerate()
                .map(|(i, f)| VariantField {
                    index: i + index_offset,
                    name: f.ident.clone(),
                    type_: f.ty.clone(),
                    span: f.span(),
                })
                .collect(),
        }
    }

    fn create_pattern(&self) -> TokenStream {
        let name = &self.name;
        let type_name = &self.type_name;
        let fields = self.fields.iter().map(|f| f.name());
        if self.fields.is_empty() {
            quote!(#type_name::#name)
        } else if self.fields[0].name.is_none() {
            quote!(#type_name::#name(#(#fields),*))
        } else {
            quote!(#type_name::#name{#(#fields),*})
        }
    }
}

#[derive(Clone)]
pub struct StructData {
    pub visibility: Visibility,
    pub name: Ident,
    pub generics: Generics,
    pub where_clause: WhereClause,
    pub rename_variants: Case<'static>,
    members: Vec<Member>,
    skipped_members: Vec<Member>,
    variant_member: Option<Member>,
    variants: Vec<VariantData>,
    pub is_partial: bool,
    pub is_row_type: bool,
    pub(crate) original_name: Ident,
}

impl StructData {
    pub(crate) fn table_name(&self) -> Ident {
        format_ident!("{}Table", self.name)
    }

    pub(crate) fn filter_name(&self) -> Ident {
        format_ident!("{}Filter", self.name)
    }

    pub(crate) fn partial_name(&self) -> Ident {
        format_ident!("Partial{}", self.name)
    }

    pub(crate) fn from_struct_data(
        visibility: Visibility,
        name: Ident,
        mut generics: Generics,
        fields: syn::Fields,
    ) -> Result<StructData, Error> {
        let fields: Vec<_> = fields
            .into_iter()
            .map(|f| (AttributeFieldData::parse(&f.attrs), f))
            .collect();
        let name_span = name.span();
        let where_clause = generics.make_where_clause().clone();
        let mut this = Self {
            visibility,
            generics,
            original_name: name.clone(),
            name,
            members: Vec::new(),
            rename_variants: Case::Snake,
            variant_member: None,
            skipped_members: Vec::new(),
            variants: Vec::new(),
            is_partial: false,
            is_row_type: false,
            where_clause,
        };
        if fields.iter().find(|f| !f.0.is_skip).is_none() {
            return Err(Error::new(name_span, crate::error::ErrorKind::NoColumns));
        }
        if let Some(multiple_primaries) = fields.iter().filter(|f| f.0.is_primary).nth(1) {
            return Err(Error::new(
                // TODO: I would like ident.span() more, but if it is a tuple
                // type, we would need its type instead.
                multiple_primaries.1.span(),
                crate::error::ErrorKind::TooManyPrimaries,
            ));
        }
        this.populate_members(fields);
        Ok(this)
    }

    pub(crate) fn from_enum_data(
        visibility: Visibility,
        name: Ident,
        mut generics: Generics,
        variants: syn::punctuated::Punctuated<syn::Variant, syn::token::Comma>,
        attrs: ToColumnsAttributesEnum,
    ) -> Result<StructData, Error> {
        let rename_variants = attrs.rename.unwrap_or(Case::Snake);
        let fields = variants
            .iter()
            .flat_map(|v| v.fields.iter())
            .map(|f| (AttributeFieldData::parse(&f.attrs), f.clone()))
            .collect();
        let mut index_offset = 0;
        let variants: Vec<_> = variants
            .iter()
            .map(|v| {
                let v = VariantData::new(name.clone(), v, index_offset);
                index_offset += v.fields.len();
                v
            })
            .collect();
        let where_clause = generics.make_where_clause().clone();

        let mut this = Self {
            variants,
            original_name: name.clone(),
            rename_variants,
            variant_member: Some(Member::create_variant_member(name.span())),
            visibility,
            name,
            generics,
            where_clause,
            members: vec![],
            skipped_members: vec![],
            is_partial: false,
            is_row_type: false,
        };
        this.populate_members(fields);
        Ok(this)
    }

    fn populate_members(&mut self, fields: Vec<(AttributeFieldData, syn::Field)>) {
        self.skipped_members = fields
            .iter()
            .enumerate()
            .filter(|(_, (a, _))| a.is_skip)
            .map(|(i, (a, f))| Member::from_field_with_attributes(i, a, f))
            .collect();
        self.members = fields
            .iter()
            .enumerate()
            .filter(|(_, (a, _))| !a.is_skip)
            .map(|(i, (a, f))| Member::from_field_with_attributes(i, a, f))
            .collect();
    }

    pub fn columns(&self) -> Vec<ColumnData<'_>> {
        self.members.iter().map(|m| m.to_column_data()).collect()
    }

    pub(crate) fn fields(&self) -> Vec<Field<'_>> {
        self.members.iter().map(|m| m.to_field()).collect()
    }

    pub(crate) fn skipped_fields(&self) -> Vec<Field<'_>> {
        self.skipped_members.iter().map(|m| m.to_field()).collect()
    }

    pub(crate) fn variant_field(&self) -> Option<Field<'_>> {
        self.variant_member.as_ref().map(|m| m.to_field())
    }
    pub(crate) fn to_partial(&self) -> StructData {
        StructData {
            visibility: self.visibility.clone(),
            original_name: self.original_name.clone(),
            name: self.partial_name(),
            generics: self.generics.clone(),
            where_clause: self.where_clause.clone(),
            members: self
                .members
                .iter()
                .cloned()
                .map(Member::to_partial)
                .collect(),
            skipped_members: self
                .skipped_members
                .iter()
                .cloned()
                .map(Member::to_partial)
                .collect(),
            rename_variants: self.rename_variants,
            variant_member: self.variant_member.clone().map(Member::to_partial),
            variants: self.variants.clone(),
            is_row_type: self.is_row_type,
            is_partial: true,
        }
    }

    pub(crate) fn variant_patterns(&self) -> Vec<TokenStream> {
        self.variants.iter().map(|v| v.create_pattern()).collect()
    }

    pub(crate) fn variants_fields(&self) -> Vec<Vec<VariantField>> {
        self.variants.iter().map(|v| v.fields.clone()).collect()
    }

    pub(crate) fn primary_key_field(&self) -> Option<Field<'_>> {
        self.members
            .iter()
            .find(|m| m.is_primary)
            .map(|m| m.to_field())
    }

    pub fn is_enum(&self) -> bool {
        !self.variants.is_empty()
    }

    pub(crate) fn is_simple_enum(&self) -> bool {
        !self.variants.is_empty() && self.variants.iter().all(|v| v.fields.is_empty())
    }

    pub(crate) fn discriminant_values(&self) -> Vec<syn::Expr> {
        self.variants
            .iter()
            .map(|v| {
                let ident = v.name.unraw().to_string();
                let lit = ident.to_case(self.rename_variants);
                syn::Expr::Lit(syn::ExprLit {
                    attrs: Vec::new(),
                    lit: syn::Lit::Str(syn::LitStr::new(&lit, v.name.span())),
                })
            })
            .collect()
    }

    pub(crate) fn generics_names_only(&self, extend_lifetimes: TokenStream) -> TokenStream {
        let lifetimes = self.generics.lifetimes().map(|l| &l.lifetime).collect_vec();
        let types = self.generics.type_params().map(|t| &t.ident).collect_vec();
        let consts = self.generics.const_params().map(|c| &c.ident).collect_vec();
        quote! {< #extend_lifetimes #(#lifetimes,)* #(#types,)* #(#consts,)*>}
    }
}

impl ToTokens for StructData {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        assert!(
            self.variant_member.is_none(),
            "Enum serialization not supported!"
        );
        let visibility = &self.visibility;
        let name = &self.name;
        let members = &self.fields();
        let iter = quote! {
            #visibility struct #name {
                #(#members,)*
            }
        };
        tokens.extend(iter);
    }
}
