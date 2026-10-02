use std::{any::type_name, borrow::Cow, marker::PhantomData, ops::RangeBounds};

use rusqlite::Connection;

use crate::{Error, ToTable, debug_sql, filter::Filter};

pub struct MaxOf<T: Into<ProjectionColumn<()>>>(pub T);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Aggregation {
    Average,
    Count,
    Max,
    Median,
    Min,
    Sum,
}
#[derive(Debug, Clone)]
pub struct ProjectionColumn<T> {
    column: Cow<'static, str>,
    aggregation: T,
}

impl ProjectionColumn<()> {
    fn write_to(&self, buf: &mut String) {
        buf.push_str(&self.column);
    }
}
impl ProjectionColumn<Aggregation> {
    fn write_to(&self, buf: &mut String) {
        let fn_name = match self.aggregation {
            Aggregation::Average => "AVG(",
            Aggregation::Count => "COUNT(",
            Aggregation::Max => "MAX(",
            Aggregation::Median => "MEDIAN(",
            Aggregation::Min => "MIN(",
            Aggregation::Sum => "SUM(",
        };
        buf.push_str(fn_name);
        buf.push_str(&self.column);
        buf.push_str(") AS ");
        buf.push_str(&self.column);
    }
}

impl<U: Into<ProjectionColumn<()>>> From<MaxOf<U>> for ProjectionColumn<Aggregation> {
    fn from(value: MaxOf<U>) -> Self {
        Self {
            column: value.0.into().column,
            aggregation: Aggregation::Max,
        }
    }
}

impl From<Cow<'static, str>> for ProjectionColumn<()> {
    fn from(value: Cow<'static, str>) -> Self {
        Self {
            column: value,
            aggregation: (),
        }
    }
}

impl From<&'static str> for ProjectionColumn<()> {
    fn from(value: &'static str) -> Self {
        Self {
            column: value.into(),
            aggregation: (),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProjectionColumns<T>(Vec<ProjectionColumn<T>>);

impl<T: Clone> ProjectionColumns<T> {
    fn sub_range<R: RangeBounds<usize>>(&self, r: R) -> Self {
        let start = match r.start_bound() {
            std::ops::Bound::Included(it) => *it,
            std::ops::Bound::Excluded(_) => todo!(),
            std::ops::Bound::Unbounded => 0,
        };
        let end = match r.end_bound() {
            std::ops::Bound::Included(_) => todo!(),
            std::ops::Bound::Excluded(it) => *it,
            std::ops::Bound::Unbounded => self.0.len(),
        };
        Self(self.0[start..end].to_vec())
    }
}

impl<U, T: Into<ProjectionColumn<U>>> From<T> for ProjectionColumns<U> {
    fn from(value: T) -> Self {
        Self(vec![value.into()])
    }
}

// impl From<&'static str> for ProjectionColumns {
//     fn from(value: &'static str) -> Self {
//         Self(vec![value.into()])
//     }
// }

impl<const N: usize> From<[Cow<'static, str>; N]> for ProjectionColumns<()> {
    fn from(value: [Cow<'static, str>; N]) -> Self {
        Self(value.map(Into::into).to_vec())
    }
}

pub trait Projectable<T>: Sized {
    const COUNT: usize;

    fn from_row(
        names: &ProjectionColumns<T>,
        row: &rusqlite::Row,
        _connection: &rusqlite::Connection,
    ) -> Result<Self, Error>;
}

macro_rules! impl_projectable_single_column {
    ($t:ty) => {
        impl<T> Projectable<T> for $t {
            const COUNT: usize = 1;

            fn from_row(
                names: &ProjectionColumns<T>,
                row: &rusqlite::Row,
                _connection: &rusqlite::Connection,
            ) -> Result<Self, Error> {
                match row.get(names.0[0].column.as_ref()) {
                    Ok(it) => Ok(it),
                    Err(rusqlite::Error::InvalidColumnName(_)) => {
                        Err(Error::MissingColumn(names.0[0].column.to_string().into()))
                    }
                    Err(rusqlite::Error::InvalidColumnType(.., t)) => {
                        Err(Error::WrongColumnType(stringify!($t).into(), t))
                    }
                    Err(err) => unreachable!("Impossible error? {err}"),
                }
            }
        }
    };
}

impl_projectable_single_column!(String);
impl_projectable_single_column!(u8);
impl_projectable_single_column!(u16);
impl_projectable_single_column!(u32);
impl_projectable_single_column!(u64);
impl_projectable_single_column!(usize);
impl_projectable_single_column!(i8);
impl_projectable_single_column!(i16);
impl_projectable_single_column!(i32);
impl_projectable_single_column!(i64);
impl_projectable_single_column!(isize);

macro_rules! impl_projectable_tuples {
    ($($t:ident),+$(,)?) => {
         impl<T: Clone, $($t,)+> Projectable<T> for ($($t,)+)
            where $($t: Projectable<T>,)+
          {
            const COUNT: usize = 0 $(+ $t::COUNT)+;

            fn from_row(
                names: &ProjectionColumns<T>,
                row: &rusqlite::Row,
                connection: &rusqlite::Connection,
            ) -> Result<Self, Error> {
                let mut count = 0;
                // This is necessary, because the last count += $t::COUNT is never used.
                #[allow(unused_assignments)]
                {
                    Ok((
                        $({
                            let value =
                            $t::from_row(&names.sub_range(count..).sub_range(..$t::COUNT), row, connection)?;
                            count += $t::COUNT;
                            value
                        },)+
                    ))

                }
            }
        }
    };
}

impl_projectable_tuples!(T1);
impl_projectable_tuples!(T1, T2);
impl_projectable_tuples!(T1, T2, T3);
impl_projectable_tuples!(T1, T2, T3, T4);
impl_projectable_tuples!(T1, T2, T3, T4, T5);
impl_projectable_tuples!(T1, T2, T3, T4, T5, T6);
impl_projectable_tuples!(T1, T2, T3, T4, T5, T6, T7);
impl_projectable_tuples!(T1, T2, T3, T4, T5, T6, T7, T8);
impl_projectable_tuples!(T1, T2, T3, T4, T5, T6, T7, T8, T9);
impl_projectable_tuples!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10);
impl_projectable_tuples!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11);
impl_projectable_tuples!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12);
impl_projectable_tuples!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13);
impl_projectable_tuples!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14);
impl_projectable_tuples!(
    T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15
);
impl_projectable_tuples!(
    T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16
);

pub struct Projection<P, T>
where
    P: Projectable<T>,
{
    columns: ProjectionColumns<T>,
    unique_only: bool,
    output: PhantomData<P>,
}

impl<P: Projectable<()>> Projection<P, ()> {
    fn columns_to_sql(&self) -> String {
        let mut buf = String::new();
        if self.unique_only {
            buf.push_str("DISTINCT ");
        }
        for (i, column) in self.columns.0.iter().enumerate() {
            if i != 0 {
                buf.push_str(", ");
            }
            column.write_to(&mut buf);
        }
        buf
    }
}

impl<P: Projectable<Aggregation>> Projection<P, Aggregation> {
    fn columns_to_sql(&self) -> String {
        let mut buf = String::new();
        if self.unique_only {
            buf.push_str("DISTINCT ");
        }
        for (i, column) in self.columns.0.iter().enumerate() {
            if i != 0 {
                buf.push_str(", ");
            }
            column.write_to(&mut buf);
        }
        buf
    }
}

impl<T, P: Projectable<T>> Projection<P, T> {
    pub(crate) fn new(columns: ProjectionColumns<T>) -> Self {
        Self {
            columns,
            unique_only: false,
            output: PhantomData,
        }
    }

    pub(crate) fn with_distinct(mut self, distinct: bool) -> Self {
        self.unique_only = distinct;
        self
    }
}

pub fn project<'a, T: ToTable<'a>, P: Projectable<()>, F: Filter>(
    connection: &Connection,
    projection: Projection<P, ()>,
    filter: impl Into<F>,
) -> rusqlite::Result<Vec<P>> {
    if projection.columns.0.len() != P::COUNT {
        panic!(
            "Mismatch between wanted columns ({}) in return type and given column names ({}). In nightly, you can enable compile time checks for this.\n\nExpected type was: {}\nGiven column names were:\n  {}",
            P::COUNT,
            projection.columns.0.len(),
            type_name::<P>(),
            projection
                .columns
                .0
                .iter()
                .map(|c| c.column.as_ref())
                .collect::<Vec<_>>()
                .join("\n  ")
        );
    }
    let filter = filter.into();
    let columns = projection.columns_to_sql();
    let mut sql = format!("SELECT {columns} FROM \"{}\" WHERE ", T::table_name());
    filter.to_sql(&mut sql, None);
    let sql = sql.trim_end_matches(" WHERE ");
    debug_sql(sql);
    let mut s = connection.prepare(sql)?;
    s.query(())?
        .mapped(|r| P::from_row(&projection.columns, r, connection).map_err(|e| todo!("{}", e)))
        .collect()
}

pub fn project_aggregated<'a, T: ToTable<'a>, P: Projectable<Aggregation>, F: Filter>(
    connection: &Connection,
    projection: Projection<P, Aggregation>,
    filter: impl Into<F>,
) -> rusqlite::Result<P> {
    if projection.columns.0.len() != P::COUNT {
        panic!(
            "Mismatch between wanted columns ({}) in return type and given column names ({}). In nightly, you can enable compile time checks for this.\n\nExpected type was: {}\nGiven column names were:\n  {}",
            P::COUNT,
            projection.columns.0.len(),
            type_name::<P>(),
            projection
                .columns
                .0
                .iter()
                .map(|c| c.column.as_ref())
                .collect::<Vec<_>>()
                .join("\n  ")
        );
    }
    let filter = filter.into();
    let columns = projection.columns_to_sql();
    let mut sql = format!("SELECT {columns} FROM \"{}\" WHERE ", T::table_name());
    filter.to_sql(&mut sql, None);
    let sql = sql.trim_end_matches(" WHERE ");
    debug_sql(sql);
    let mut s = connection.prepare(sql)?;
    Ok(s.query_one((), |r| {
        P::from_row(&projection.columns, r, connection).map_err(|e| todo!("{}", e))
    })?)
}
