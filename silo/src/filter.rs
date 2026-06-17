use crate::{AsParams, ToSqlDyn, conversions::ToSqlValueString};
use chrono::{DateTime, Utc};
use std::fmt::Write;
use uuid::{NonNilUuid, Uuid};

pub enum FieldFilter<T: IsFieldFilter> {
    None,
    Not(Box<FieldFilter<T>>),
    Contains(T),
    Equals(T),
}

impl<T: IsFieldFilter> FieldFilter<T> {
    pub fn contains_not(t: &T) -> Self {
        Self::not(Self::contains(t))
    }

    pub fn contains(t: &T) -> Self {
        Self::Contains(t.clone())
    }

    pub fn equals(t: &T) -> Self {
        Self::Equals(t.clone())
    }

    pub fn not(f: FieldFilter<T>) -> FieldFilter<T> {
        Self::Not(Box::new(f))
    }
}

impl<T: IsFieldFilter> AsParams for FieldFilter<T> {
    fn as_params<'b>(&'b self) -> Vec<crate::ToSqlDyn<'b>> {
        match self {
            FieldFilter::None => Vec::new(),
            FieldFilter::Not(field_filter) => field_filter.as_params(),
            FieldFilter::Contains(it) | FieldFilter::Equals(it) => vec![ToSqlDyn::Borrowed(it)],
        }
    }
}

impl<T: IsFieldFilter> Default for FieldFilter<T> {
    fn default() -> Self {
        Self::None
    }
}

pub trait Filter: AsParams {
    fn to_sql(&self, sql: &mut String, parent: Option<&str>);
}

impl<T: IsFieldFilter> Filter for FieldFilter<T> {
    fn to_sql(&self, sql: &mut String, parent: Option<&str>) {
        match self {
            FieldFilter::None => {}
            FieldFilter::Not(field_filter) => {
                ensure_where_or_and(sql);
                _ = write!(sql, " NOT (");
                field_filter.to_sql(sql, parent);
                _ = write!(sql, ")");
            }
            FieldFilter::Contains(it) => {
                ensure_where_or_and(sql);
                <T as IsFieldFilter>::to_sql(
                    it,
                    sql,
                    ComparisonOperator::Like,
                    parent.expect("Needs a column name for comparison."),
                );
            }
            FieldFilter::Equals(it) => {
                ensure_where_or_and(sql);
                <T as IsFieldFilter>::to_sql(
                    it,
                    sql,
                    ComparisonOperator::Equals,
                    parent.expect("Needs a column name for comparison."),
                );
            }
        }
    }
}

fn ensure_where_or_and(sql: &mut String) {
    if !["AND", "(", "WHERE"]
        .into_iter()
        .any(|s| sql.trim().ends_with(s))
    {
        _ = write!(sql, " AND")
    }
}

pub trait Filterable {
    type Filter: Filter;

    fn convert_to_equals_filter(self) -> Self::Filter;
}

macro_rules! impl_filterable {
    ($t:ty) => {
        impl Filterable for $t {
            type Filter = FieldFilter<$t>;
            fn convert_to_equals_filter(self) -> Self::Filter {
                FieldFilter::Equals(self)
            }
        }

        impl IsFieldFilter for $t {
            fn to_sql(&self, sql: &mut String, operator: ComparisonOperator, parent: &str) {
                _ = write!(sql, "{parent} {operator} ");
                self.write_to_sql(sql, operator);
            }
        }
    };
    ($t:ty, $f:ty) => {
        impl Filterable for $t {
            type Filter = FieldFilter<$f>;
            fn convert_to_equals_filter(self) -> Self::Filter {
                FieldFilter::Equals(self.to_sql_value_string())
            }
        }
    };
}

impl_filterable!(DateTime<Utc>, String);
impl_filterable!(NonNilUuid, String);
impl_filterable!(Uuid, String);
impl_filterable!(String);
impl_filterable!(bool);
impl_filterable!(u8);
impl_filterable!(u64);

macro_rules! impl_write_to_sql_as_to_string {
    ($t:ty) => {
        impl WriteToSql for $t {
            fn write_to_sql(&self, sql: &mut String, _operator: ComparisonOperator) {
                _ = write!(sql, "{self}");
            }
        }
    };
}

impl_write_to_sql_as_to_string!(u8);
impl_write_to_sql_as_to_string!(u64);

impl WriteToSql for bool {
    fn write_to_sql(&self, sql: &mut String, _operator: ComparisonOperator) {
        _ = write!(sql, "{}", *self as usize);
    }
}
impl WriteToSql for String {
    fn write_to_sql(&self, sql: &mut String, operator: ComparisonOperator) {
        let surroundings = match operator {
            ComparisonOperator::Like => "%",
            _ => "",
        };
        _ = write!(sql, "'{surroundings}{self}{surroundings}'");
    }
}

#[derive(Debug, strum::Display)]
pub enum ComparisonOperator {
    #[strum(to_string = "=")]
    Equals,
    #[strum(to_string = "LIKE")]
    Like,
}

pub trait WriteToSql {
    fn write_to_sql(&self, sql: &mut String, operator: ComparisonOperator);
}

pub trait IsFieldFilter: rusqlite::ToSql + Clone + WriteToSql {
    fn to_sql(&self, sql: &mut String, operator: ComparisonOperator, parent: &str);
}
