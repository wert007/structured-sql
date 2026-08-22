use crate::{AsParams, Blob, ToSqlDyn, conversions::ToSqlValueString};
use chrono::{DateTime, Utc};
use std::{borrow::Cow, fmt::Write};
use time::{Date, OffsetDateTime, Time};
use uuid::{NonNilUuid, Uuid};

#[derive(Default)]
pub struct NoopFilter;

impl AsParams for NoopFilter {
    fn as_params<'b>(&'b self) -> Vec<ToSqlDyn<'b>> {
        Vec::new()
    }
}

impl Filter for NoopFilter {
    fn to_sql(&self, _sql: &mut String, _parent: Option<&str>) {}
}

#[derive(Default)]
pub enum OptionalFilter<T: Filter> {
    #[default]
    IsEither,
    IsNone,
    IsSome,
    IsSomeAnd(T),
}

impl<T: Filter> AsParams for OptionalFilter<T> {
    fn as_params<'b>(&'b self) -> Vec<ToSqlDyn<'b>> {
        match self {
            OptionalFilter::IsEither | OptionalFilter::IsNone | OptionalFilter::IsSome => {
                Vec::new()
            }
            OptionalFilter::IsSomeAnd(it) => it.as_params(),
        }
    }
}

impl<T: Filter> Filter for OptionalFilter<T> {
    fn to_sql(&self, sql: &mut String, parent: Option<&str>) {
        match self {
            OptionalFilter::IsEither => {}
            OptionalFilter::IsNone => todo!(),
            OptionalFilter::IsSome => todo!(),
            OptionalFilter::IsSomeAnd(it) => it.to_sql(sql, parent),
        }
    }
}

#[derive(Default)]
pub enum FieldFilter<T: IsFieldFilter> {
    #[default]
    None,
    Not(Box<FieldFilter<T>>),
    Comparison(T, ComparisonOperator),
    Or(Vec<FieldFilter<T>>),
}

impl<T: IsFieldFilter> FieldFilter<T> {
    pub fn contains_not(t: &T) -> Self {
        Self::not(Self::contains(t))
    }

    pub fn or(cases: impl Iterator<Item = Self>) -> Self {
        Self::Or(cases.collect())
    }

    pub fn contains(t: &T) -> Self {
        Self::Comparison(t.clone(), ComparisonOperator::Like)
    }

    pub fn equals(t: impl Into<T>) -> Self {
        Self::Comparison(t.into(), ComparisonOperator::Equals)
    }

    pub fn greater_than(t: impl Into<T>) -> Self {
        Self::Comparison(t.into(), ComparisonOperator::GreaterThan)
    }

    pub fn greater_than_equals(t: impl Into<T>) -> Self {
        Self::Comparison(t.into(), ComparisonOperator::GreaterThanEquals)
    }

    pub fn less_than(t: impl Into<T>) -> Self {
        Self::Comparison(t.into(), ComparisonOperator::LessThan)
    }

    pub fn less_than_equals(t: impl Into<T>) -> Self {
        Self::Comparison(t.into(), ComparisonOperator::LessThanEquals)
    }

    pub fn not(f: FieldFilter<T>) -> Self {
        Self::Not(Box::new(f))
    }
}

impl<T: IsFieldFilter> AsParams for FieldFilter<T> {
    fn as_params<'b>(&'b self) -> Vec<crate::ToSqlDyn<'b>> {
        match self {
            FieldFilter::None => Vec::new(),
            FieldFilter::Not(field_filter) => field_filter.as_params(),
            FieldFilter::Comparison(it, _) => {
                vec![ToSqlDyn::Borrowed(it)]
            }
            FieldFilter::Or(field_filters) => field_filters
                .iter()
                .flat_map(|f| f.as_params().into_iter())
                .collect(),
        }
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
                _ = write!(sql, "NOT (");
                field_filter.to_sql(sql, parent);
                _ = write!(sql, ")");
            }
            FieldFilter::Comparison(it, operator) => {
                ensure_where_or_and(sql);
                <T as IsFieldFilter>::to_sql(
                    it,
                    sql,
                    *operator,
                    parent.expect("Needs a column name for comparison."),
                );
            }
            FieldFilter::Or(field_filters) => {
                ensure_where_or_and(sql);
                _ = write!(sql, " (");
                for (i, f) in field_filters.iter().enumerate() {
                    if i > 0 {
                        _ = write!(sql, " OR ");
                    }
                    f.to_sql(sql, parent);
                }
                _ = write!(sql, " )");
            }
        }
    }
}

fn ensure_where_or_and(sql: &mut String) {
    if !["AND", "(", "WHERE", "OR"]
        .into_iter()
        .any(|s| sql.trim().ends_with(s))
    {
        _ = write!(sql, " AND ")
    }
}

pub trait Filterable {
    type Filter: Filter + Default;

    fn convert_to_equals_filter(self) -> Self::Filter;
}

impl<T: Filterable> Filterable for Option<T> {
    type Filter = OptionalFilter<T::Filter>;

    fn convert_to_equals_filter(self) -> Self::Filter {
        match self {
            Some(it) => OptionalFilter::IsSomeAnd(it.convert_to_equals_filter()),
            None => OptionalFilter::IsNone,
        }
    }
}

macro_rules! impl_filterable {
    ($t:ty) => {
        impl Filterable for $t {
            type Filter = FieldFilter<$t>;
            fn convert_to_equals_filter(self) -> Self::Filter {
                FieldFilter::equals(self)
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
                FieldFilter::equals(self.to_sql_value_string())
            }
        }
    };
}

impl_filterable!(DateTime<Utc>, String);
impl_filterable!(Time, String);
impl_filterable!(Date, String);
impl_filterable!(OffsetDateTime, String);
impl_filterable!(NonNilUuid, String);
impl_filterable!(Uuid, String);
impl_filterable!(String);
impl_filterable!(bool);
impl_filterable!(u8);
impl_filterable!(u16);
impl_filterable!(u32);
impl_filterable!(u64);
impl_filterable!(usize);
impl_filterable!(i8);
impl_filterable!(i16);
impl_filterable!(i32);
impl_filterable!(i64);
impl_filterable!(isize);
// We might wanna use a manual implementation later, to be closer to sqlite? To
// be able to work around floating point shenanigans.
impl_filterable!(f32);
impl_filterable!(f64);

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
impl_write_to_sql_as_to_string!(u16);
impl_write_to_sql_as_to_string!(u32);
impl_write_to_sql_as_to_string!(u64);
impl_write_to_sql_as_to_string!(usize);
impl_write_to_sql_as_to_string!(i8);
impl_write_to_sql_as_to_string!(i16);
impl_write_to_sql_as_to_string!(i32);
impl_write_to_sql_as_to_string!(i64);
impl_write_to_sql_as_to_string!(isize);
impl_write_to_sql_as_to_string!(f32);
impl_write_to_sql_as_to_string!(f64);

impl Filterable for Blob {
    type Filter = NoopFilter;

    fn convert_to_equals_filter(self) -> Self::Filter {
        NoopFilter
    }
}

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
        _ = write!(sql, "'{surroundings}{}{surroundings}'", escape_sql(self));
    }
}

fn escape_sql(p: &'_ str) -> Cow<'_, str> {
    if p.contains(['\'', '\\', '\n', '\r', '\t', '\0']) {
        p.chars()
            .flat_map(|c: char| -> smallvec::SmallVec<[char; 2]> {
                match c {
                    '\'' => smallvec::smallvec!['\'', '\''],
                    '\\' => smallvec::smallvec!['\\', '\\'],
                    '\n' => smallvec::smallvec!['\\', 'n'],
                    '\r' => smallvec::smallvec!['\\', 'r'],
                    '\t' => smallvec::smallvec!['\\', 't'],
                    '\0' => smallvec::smallvec!['\\', '0'],
                    _ => smallvec::smallvec![c],
                }
            })
            .collect::<String>()
            .into()
    } else {
        p.into()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::Display)]
pub enum ComparisonOperator {
    #[strum(to_string = "=")]
    Equals,
    #[strum(to_string = ">")]
    GreaterThan,
    #[strum(to_string = ">=")]
    GreaterThanEquals,
    #[strum(to_string = "<")]
    LessThan,
    #[strum(to_string = "<=")]
    LessThanEquals,
    #[strum(to_string = "LIKE")]
    Like,
}

pub trait WriteToSql {
    fn write_to_sql(&self, sql: &mut String, operator: ComparisonOperator);
}

pub trait IsFieldFilter: rusqlite::ToSql + Clone + WriteToSql {
    fn to_sql(&self, sql: &mut String, operator: ComparisonOperator, parent: &str);
}
