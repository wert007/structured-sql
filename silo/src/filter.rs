use std::borrow::Cow;
use std::fmt::Write;

pub trait ToFilter: Default {
    fn to_filter(self) -> GenericFilter;
}

#[derive(Default)]
pub enum GenericFilter {
    #[default]
    None,
    And(Vec<GenericFilter>),
    Or(Vec<GenericFilter>),
    Not(Box<GenericFilter>),
    Field(FieldFilter),
}
impl GenericFilter {
    pub(crate) fn write_to(&self, sql: &mut String, needs_where: bool) {
        match self {
            GenericFilter::None => {}
            GenericFilter::And(generic_filters) => {
                if needs_where {
                    write!(sql, " WHERE ").unwrap();
                }
                write!(sql, "(").unwrap();
                for (i, and) in generic_filters.iter().enumerate() {
                    if i != 0 {
                        write!(sql, " AND ").unwrap();
                    }
                    and.write_to(sql, false);
                }
                write!(sql, ")").unwrap();
            }
            GenericFilter::Or(generic_filters) => {
                if needs_where {
                    write!(sql, " WHERE ").unwrap();
                }
                write!(sql, "(").unwrap();
                for (i, or) in generic_filters.iter().enumerate() {
                    if i != 0 {
                        write!(sql, " OR ").unwrap();
                    }
                    or.write_to(sql, false);
                }
                write!(sql, ")").unwrap();
            }
            GenericFilter::Not(not) => {
                if needs_where {
                    write!(sql, " WHERE ").unwrap();
                }

                write!(sql, "NOT ").unwrap();
                not.write_to(sql, false);
            }
            GenericFilter::Field(field_filter) => {
                if needs_where {
                    write!(sql, " WHERE ").unwrap();
                }
                field_filter.write_to(sql);
            }
        }
    }
}

pub struct FieldFilter {
    pub field: Cow<'static, str>,
    pub value: String,
    pub operator: FilterOperator,
}

impl FieldFilter {
    fn write_to(&self, sql: &mut String) {
        let operator = match self.operator {
            FilterOperator::Equals => "=",
            FilterOperator::NotEquals => "!=",
            FilterOperator::LessThan => "<",
            FilterOperator::LessThanEquals => "<=",
            FilterOperator::GreaterThan => ">",
            FilterOperator::GreaterThanEquals => ">=",
            FilterOperator::Like => "LIKE",
            FilterOperator::Glob => "GLOB",
        };
        write!(sql, "{} {operator} {}", &self.field, &self.value).unwrap();
    }
}

pub enum FilterOperator {
    Equals,
    NotEquals,
    LessThan,
    LessThanEquals,
    GreaterThan,
    GreaterThanEquals,
    Like,
    Glob,
}
