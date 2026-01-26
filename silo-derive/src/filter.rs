use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Type, parse_quote};

use crate::type_checker::TypeIsLike;

pub(crate) fn create_filter_for(
    base_struct: &crate::base_struct::StructData,
) -> proc_macro2::TokenStream {
    let visibility = &base_struct.visibility;
    let filter_name = base_struct.filter_name();
    let mut filter_functions = quote!();
    for field in base_struct.fields() {
        for operator in get_operators_for_type(field.type_) {
            let fn_name = format_ident!("{}_{}", field.name, operator.fn_name);
            let type_ = &operator.argument_type;
            let field_name = field.name;
            let value_conversion = &operator.value_conversion;
            let filter_operator = operator.filter_operator;
            filter_functions.extend(quote! {
                fn #fn_name(&self, value: #type_) -> Self {
                    let value = #value_conversion;
                    Self {
                        generic: silo::filter::GenericFilter::Field(silo::filter::FieldFilter {
                            field: stringify!(#field_name).into(),
                            value,
                            operator: #filter_operator,
                        })
                    }
                }
            });
        }
    }
    quote! {
        #[derive(Default)]
        #visibility struct #filter_name {
            generic: silo::filter::GenericFilter
        }

        impl #filter_name {
            fn or(&self, lhs: Self, rhs: Self) -> Self {
                Self {
                    generic: silo::filter::GenericFilter::Or(vec![lhs.generic, rhs.generic]),
                }
            }

            fn and(&self, lhs: Self, rhs: Self) -> Self {
                Self {
                    generic: silo::filter::GenericFilter::And(vec![lhs.generic, rhs.generic]),
                }
            }

            #filter_functions
        }

        impl silo::filter::ToFilter for #filter_name {
            fn to_filter(self) -> silo::filter::GenericFilter {
                self.generic
            }
        }

    }
}

struct FilterOperator {
    argument_type: Type,
    fn_name: &'static str,
    filter_operator: TokenStream,
    value_conversion: TokenStream,
}

fn get_operators_for_type(type_: &syn::Type) -> Vec<FilterOperator> {
    if type_.is_string_like() {
        vec![
            FilterOperator {
                argument_type: syn::parse_quote!(impl AsRef<str>),
                fn_name: "equals",
                filter_operator: quote!(silo::filter::FilterOperator::Equals),
                value_conversion: quote!(format!("\"{}\"", value.as_ref())),
            },
            FilterOperator {
                argument_type: syn::parse_quote!(impl AsRef<str>),
                fn_name: "not_equals",
                filter_operator: quote!(silo::filter::FilterOperator::NotEquals),
                value_conversion: quote!(format!("\"{}\"", value.as_ref())),
            },
            FilterOperator {
                argument_type: syn::parse_quote!(impl AsRef<str>),
                fn_name: "starts_with",
                filter_operator: quote!(silo::filter::FilterOperator::Like),
                value_conversion: quote!(format!("\"{}%\"", value.as_ref())),
            },
            FilterOperator {
                argument_type: syn::parse_quote!(impl AsRef<str>),
                fn_name: "ends_with",
                filter_operator: quote!(silo::filter::FilterOperator::Like),
                value_conversion: quote!(format!("\"%{}\"", value.as_ref())),
            },
        ]
    } else if type_.is_number_like() {
        vec![
            FilterOperator {
                argument_type: type_.clone(),
                fn_name: "equals",
                filter_operator: quote!(silo::filter::FilterOperator::Equals),
                value_conversion: quote!(format!("{}", value)),
            },
            FilterOperator {
                argument_type: type_.clone(),
                fn_name: "not_equals",
                filter_operator: quote!(silo::filter::FilterOperator::NotEquals),
                value_conversion: quote!(format!("{}", value)),
            },
            FilterOperator {
                argument_type: type_.clone(),
                fn_name: "less_than",
                filter_operator: quote!(silo::filter::FilterOperator::LessThan),
                value_conversion: quote!(format!("{}", value)),
            },
            FilterOperator {
                argument_type: type_.clone(),
                fn_name: "less_than_equals",
                filter_operator: quote!(silo::filter::FilterOperator::LessThanEquals),
                value_conversion: quote!(format!("{}", value)),
            },
            FilterOperator {
                argument_type: type_.clone(),
                fn_name: "greater_than",
                filter_operator: quote!(silo::filter::FilterOperator::GreaterThan),
                value_conversion: quote!(format!("{}", value)),
            },
            FilterOperator {
                argument_type: type_.clone(),
                fn_name: "greater_than_equals",
                filter_operator: quote!(silo::filter::FilterOperator::GreaterThanEquals),
                value_conversion: quote!(format!("{}", value)),
            },
        ]
    } else if type_.is_bool_like() {
        vec![
            FilterOperator {
                argument_type: type_.clone(),
                fn_name: "equals",
                filter_operator: quote!(silo::filter::FilterOperator::Equals),
                value_conversion: quote!(format!("{}", if value { 1 } else { 0 })),
            },
            FilterOperator {
                argument_type: type_.clone(),
                fn_name: "not_equals",
                filter_operator: quote!(silo::filter::FilterOperator::NotEquals),
                value_conversion: quote!(format!("{}", if value { 1 } else { 0 })),
            },
        ]
    } else {
        Vec::new()
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
