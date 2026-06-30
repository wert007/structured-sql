use std::{any::type_name, borrow::Cow, marker::PhantomData, ops::RangeBounds};

use rusqlite::Connection;

use crate::{Error, ToTable, debug_sql, filter::Filter};

pub struct ProjectionColumns(Vec<Cow<'static, str>>);

impl ProjectionColumns {
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

impl From<Cow<'static, str>> for ProjectionColumns {
    fn from(value: Cow<'static, str>) -> Self {
        Self(vec![value])
    }
}

impl<const N: usize> From<[Cow<'static, str>; N]> for ProjectionColumns {
    fn from(value: [Cow<'static, str>; N]) -> Self {
        Self(value.into())
    }
}

pub trait Projectable: Sized {
    const COUNT: usize;

    fn from_row(
        names: &ProjectionColumns,
        row: &rusqlite::Row,
        _connection: &rusqlite::Connection,
    ) -> Result<Self, Error>;
}

macro_rules! impl_projectable_single_column {
    ($t:ty) => {
        impl Projectable for $t {
            const COUNT: usize = 1;

            fn from_row(
                names: &ProjectionColumns,
                row: &rusqlite::Row,
                _connection: &rusqlite::Connection,
            ) -> Result<Self, Error> {
                match row.get(names.0[0].as_ref()) {
                    Ok(it) => Ok(it),
                    Err(rusqlite::Error::InvalidColumnName(_)) => {
                        Err(Error::MissingColumn(names.0[0].to_string().into()))
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

macro_rules! impl_projectable_tuples {
    ($($t:ident),+$(,)?) => {
         impl<$($t,)+> Projectable for ($($t,)+)
            where $($t: Projectable,)+
          {
            const COUNT: usize = 0 $(+ $t::COUNT)+;

            fn from_row(
                names: &ProjectionColumns,
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

pub struct Projection<P>
where
    P: Projectable,
{
    columns: ProjectionColumns,
    unique_only: bool,
    output: PhantomData<P>,
}
impl<P: Projectable> Projection<P> {
    fn columns_to_sql(&self) -> String {
        let mut buf = String::new();
        if self.unique_only {
            buf.push_str("DISTINCT ");
        }
        for (i, column) in self.columns.0.iter().enumerate() {
            if i != 0 {
                buf.push_str(", ");
            }
            buf.push_str(column);
        }
        buf
    }

    pub(crate) fn new(columns: ProjectionColumns) -> Self {
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

pub fn project<'a, T: ToTable<'a>, P: Projectable, F: Filter>(
    connection: &Connection,
    projection: Projection<P>,
    filter: impl Into<F>,
) -> rusqlite::Result<Vec<P>> {
    if projection.columns.0.len() != P::COUNT {
        panic!(
            "Mismatch between wanted columns ({}) in return type and given column names ({}). In nightly, you can enable compile time checks for this.\n\nExpected type was: {}\nGiven column names were:\n  {}",
            P::COUNT,
            projection.columns.0.len(),
            type_name::<P>(),
            projection.columns.0.join("\n  ")
        );
    }
    let filter = filter.into();
    let columns = projection.columns_to_sql();
    let mut sql = format!("SELECT {columns} FROM {} WHERE ", T::NAME);
    filter.to_sql(&mut sql, None);
    let sql = sql.trim_end_matches(" WHERE ");
    debug_sql(sql);
    let mut s = connection.prepare(sql)?;
    s.query(())?
        .mapped(|r| P::from_row(&projection.columns, r, connection).map_err(|e| todo!("{}", e)))
        .collect()
}
