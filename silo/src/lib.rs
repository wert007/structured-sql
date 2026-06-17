use std::{
    borrow::Cow,
    fmt::Debug,
    path::Path,
    sync::atomic::{AtomicBool, Ordering::SeqCst},
};

use chrono::{DateTime, Utc};
pub use rusqlite;
use rusqlite::{Connection, ErrorCode, Params, types::Null};

mod error;
pub mod partial;
pub use error::Error;
mod conversions;
pub mod filter;

pub static DEBUG_SQL: AtomicBool = AtomicBool::new(false);

#[cfg(feature = "enable_debug_sql")]
pub fn toggle_debug_sql() {
    DEBUG_SQL.update(SeqCst, SeqCst, |b| !b);
}

fn debug_sql(sql: &str) {
    if !DEBUG_SQL.load(SeqCst) {
        return;
    };
    eprintln!("[silo] Executing {sql}");
}

// #[derive(Clone)]
pub enum ToSqlDyn<'a> {
    Boxed(Box<dyn rusqlite::ToSql>),
    Borrowed(&'a dyn rusqlite::ToSql),
}

impl<'a> ToSqlDyn<'a> {
    fn as_dyn<'b: 'a>(&'b self) -> &'a dyn rusqlite::ToSql {
        match self {
            ToSqlDyn::Boxed(to_sql) => to_sql,
            ToSqlDyn::Borrowed(to_sql) => to_sql,
        }
    }
}

impl ToSqlDyn<'static> {
    fn create_static(v: &'static dyn rusqlite::ToSql) -> Self {
        Self::Borrowed(v)
    }
}

pub mod derive {
    pub use silo_derive::ToTable;
}

use time::macros::format_description;
use time::{Date, Time};
use time::{OffsetDateTime, format_description::FormatItem};
use uuid::{NonNilUuid, Uuid};

pub struct Database {
    connection: rusqlite::Connection,
}

fn execute<P: Params>(
    connection: &rusqlite::Connection,
    sql: &str,
    params: P,
) -> Result<usize, rusqlite::Error> {
    debug_sql(sql);
    connection.execute(sql, params)
}

impl Database {
    fn new_from_connection(connection: rusqlite::Connection) -> Self {
        Self { connection }
    }

    pub unsafe fn from_connection(
        connection: &rusqlite::Connection,
    ) -> Result<Self, rusqlite::Error> {
        let connection = unsafe { rusqlite::Connection::from_handle(connection.handle())? };
        Ok(Self::new_from_connection(connection))
    }

    pub fn create_in_memory() -> Result<Self, rusqlite::Error> {
        let connection = rusqlite::Connection::open_in_memory()?;
        execute(&connection, "DROP TABLE IF EXISTS temporary", ())?;
        Ok(Self::new_from_connection(connection))
    }

    pub fn open(path: impl AsRef<Path>) -> Result<Self, rusqlite::Error> {
        let connection = rusqlite::Connection::open(path)?;
        execute(&connection, "DROP TABLE IF EXISTS temporary", ())?;
        Ok(Self::new_from_connection(connection))
    }

    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), rusqlite::Error> {
        self.connection.backup("main", path, None)?;
        Ok(())
    }
    pub fn load<'a, T: ToTable<'a>>(&'a self) -> rusqlite::Result<T::Table> {
        self.create::<T>()?;

        Ok(T::Table::from_connection(&self.connection))
    }

    fn create<'a, T: ToTable<'a>>(&'a self) -> Result<(), rusqlite::Error> {
        if self.connection.table_exists(None, T::NAME)? {
            return Ok(());
        }
        let mut sql = "CREATE TABLE IF NOT EXISTS ".to_string();
        sql.push_str(T::NAME);
        sql.push_str(" (");
        for (i, column) in T::columns(None, false, false).into_iter().enumerate() {
            if i > 0 {
                sql.push_str(",");
            }
            sql.push('"');
            sql.push_str(&column.name);
            sql.push('"');
            sql.push_str(" ");
            sql.push_str(column.r#type.as_sql());
            if column.is_unique {
                sql.push_str(" UNIQUE");
            }
            if column.is_primary {
                sql.push_str(" PRIMARY KEY");
            }
        }
        sql.push_str(");");
        debug_sql(&sql);

        self.connection.execute(&sql, ())?;
        Ok(())
    }
}

/// This trait represents the columns, that may be part of a struct. Each Column
/// has all information needed to create the correct table, given the table
/// name. But this does not need to be a table. You could e.g. have a
/// Coordinate, which would have to columns, which might be just a part of a
/// different struct somewhere else.
pub trait AsColumns: AsColumnsDynamicallySized {
    const COLUMN_COUNT: usize;
}

/// This part actually generates the columns. Right now the differentiation
/// between AsColumns and AsColumnsDynamicallySized is not important at all and
/// is never used. But it has been prepared in case it is needed at some point.
pub trait AsColumnsDynamicallySized {
    fn columns(parent: Option<&str>, is_unique: bool, is_primary: bool) -> Vec<SqlColumn>;
}

/// This trait turns an actual value into all the params (Arguments) that
/// rusqlite would take to fill in ?1.
pub trait AsParams {
    fn as_params<'b>(&'b self) -> Vec<ToSqlDyn<'b>>;
}

/// This trait will skip fields, where its value are None, this allows for
/// [`Partial`] Updates.
pub trait AsColumnsOptional {
    fn columns_skip_optional(
        &self,
        parent: Option<&str>,
        is_unique: bool,
        is_primary: bool,
    ) -> Vec<SqlColumn>;
}

/// This trait will skip fields, where its value are None, this allows for
/// [`Partial`] Updates.
pub trait AsParamsOptional {
    fn as_params_skip_optional<'b>(&'b self) -> Vec<ToSqlDyn<'b>>;
}

impl<T: AsColumns> AsColumnsOptional for Option<T> {
    fn columns_skip_optional(
        &self,
        parent: Option<&str>,
        is_unique: bool,
        is_primary: bool,
    ) -> Vec<SqlColumn> {
        match self {
            Some(_) => T::columns(parent, is_unique, is_primary),
            None => Vec::new(),
        }
    }
}

impl<T: AsParams> AsParamsOptional for Option<T> {
    fn as_params_skip_optional<'b>(&'b self) -> Vec<ToSqlDyn<'b>> {
        match self {
            Some(it) => it.as_params(),
            None => Vec::new(),
        }
    }
}

impl<T: AsParams + AsColumns> AsParams for Option<T> {
    fn as_params<'b>(&'b self) -> Vec<ToSqlDyn<'b>> {
        match self {
            Some(it) => it.as_params(),
            None => (0..T::COLUMN_COUNT)
                .into_iter()
                .map(|_| ToSqlDyn::create_static(&Null))
                .collect(),
        }
    }
}

impl<T: AsColumns> AsColumns for Option<T> {
    const COLUMN_COUNT: usize = T::COLUMN_COUNT;
}

impl<T: AsColumnsDynamicallySized> AsColumnsDynamicallySized for Option<T> {
    fn columns(parent: Option<&str>, is_unique: bool, is_primary: bool) -> Vec<SqlColumn> {
        T::columns(parent, is_unique, is_primary)
    }
}

/// This type can be represented in a single sql column. This also implements
/// AsColumns for free.
pub trait IsSingleColumn {
    const SQL_COLUMN_TYPE: SqlColumnType;
}

impl<T: IsSingleColumn> AsColumns for T {
    const COLUMN_COUNT: usize = 1;
}

impl<T: IsSingleColumn> AsColumnsDynamicallySized for T {
    fn columns(parent: Option<&str>, is_unique: bool, is_primary: bool) -> Vec<SqlColumn> {
        vec![SqlColumn {
            name: parent.unwrap().to_string().into(),
            r#type: T::SQL_COLUMN_TYPE,
            is_primary,
            is_unique,
        }]
    }
}

macro_rules! impl_as_params {
    ($t:ty, $column_type:expr) => {
        impl_as_params_base!($t, $column_type);

        impl<'a> AsParams for $t {
            fn as_params<'b>(&'b self) -> Vec<ToSqlDyn<'b>> {
                vec![ToSqlDyn::Borrowed(self)]
            }
        }

        impl<'a> ExtractFromRow for $t {
            fn try_from_row_simple(column_name: &str, row: &rusqlite::Row) -> Result<Self, Error> {
                match row.get(column_name) {
                    Ok(it) => Ok(it),
                    Err(rusqlite::Error::InvalidColumnName(_)) => {
                        Err(Error::MissingColumn(column_name.to_string().into()))
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

macro_rules! impl_as_params_base {
    ($t:ty, $column_type:expr) => {
        impl<'a> partial::HasPartial for $t {
            type Partial = Option<$t>;
        }

        impl<'a> IsSingleColumn for $t {
            const SQL_COLUMN_TYPE: SqlColumnType = $column_type;
        }
    };
}

impl_as_params!(bool, SqlColumnType::Integer);
impl_as_params!(i8, SqlColumnType::Integer);
impl_as_params!(i16, SqlColumnType::Integer);
impl_as_params!(i32, SqlColumnType::Integer);
impl_as_params!(i64, SqlColumnType::Integer);
impl_as_params!(isize, SqlColumnType::Integer);
impl_as_params!(u8, SqlColumnType::Integer);
impl_as_params!(u16, SqlColumnType::Integer);
impl_as_params!(u32, SqlColumnType::Integer);
impl_as_params!(usize, SqlColumnType::Integer);
impl_as_params!(u64, SqlColumnType::Integer);
impl_as_params!(Time, SqlColumnType::Text);
impl_as_params!(Date, SqlColumnType::Text);
impl_as_params!(DateTime<Utc>, SqlColumnType::Text);
impl_as_params_base!(NonNilUuid, SqlColumnType::Text);
impl_as_params_base!(Uuid, SqlColumnType::Text);
impl AsParams for Uuid {
    fn as_params<'b>(&'b self) -> Vec<ToSqlDyn<'b>> {
        vec![ToSqlDyn::Boxed(Box::new(self.to_string()))]
    }
}

impl AsParams for NonNilUuid {
    fn as_params<'b>(&'b self) -> Vec<ToSqlDyn<'b>> {
        vec![ToSqlDyn::Boxed(Box::new(self.get().to_string()))]
    }
}

impl ExtractFromRow for Uuid {
    fn try_from_row_simple(column_name: &str, row: &rusqlite::Row) -> Result<Self, Error> {
        match row.get::<&str, String>(column_name) {
            Ok(it) => Ok(Uuid::try_parse(&it)
                .map_err(|e| Error::IllFormattedColumn("Uuid".into(), it, Some(Box::new(e))))?),
            Err(rusqlite::Error::InvalidColumnName(_)) => {
                Err(Error::MissingColumn(column_name.to_string().into()))
            }
            Err(rusqlite::Error::InvalidColumnType(.., t)) => {
                Err(Error::WrongColumnType("Uuid".into(), t))
            }
            Err(err) => unreachable!("Impossible error? {err}"),
        }
    }
}

impl ExtractFromRow for NonNilUuid {
    fn try_from_row_simple(column_name: &str, row: &rusqlite::Row) -> Result<Self, Error> {
        match row.get::<&str, String>(column_name) {
            Ok(it) => Ok(NonNilUuid::new(Uuid::try_parse(&it).map_err(|e| {
                Error::IllFormattedColumn("Uuid".into(), it.clone(), Some(Box::new(e)))
            })?)
            .ok_or(Error::IllFormattedColumn("NonNilUuid".into(), it, None))?),
            Err(rusqlite::Error::InvalidColumnName(_)) => {
                Err(Error::MissingColumn(column_name.to_string().into()))
            }
            Err(rusqlite::Error::InvalidColumnType(.., t)) => {
                Err(Error::WrongColumnType("Uuid".into(), t))
            }
            Err(err) => unreachable!("Impossible error? {err}"),
        }
    }
}

impl_as_params!(OffsetDateTime, SqlColumnType::Text);
impl_as_params!(f32, SqlColumnType::Float);
impl_as_params!(f64, SqlColumnType::Float);
impl_as_params!(String, SqlColumnType::Text);

pub trait FromRow: Sized {
    fn try_from_row(row: &rusqlite::Row, connection: &rusqlite::Connection) -> Result<Self, Error>;
}

pub trait ExtractFromRow: Sized {
    fn try_from_row_simple(column_name: &str, row: &rusqlite::Row) -> Result<Self, Error>;
    fn try_from_row(
        column_name: &str,
        row: &rusqlite::Row,
        _connection: &rusqlite::Connection,
    ) -> Result<Self, Error> {
        Self::try_from_row_simple(column_name, row)
    }
}

// TODO: Is this right? Kind of depends on the reason of failure, doesn't it?
impl<T: ExtractFromRow> ExtractFromRow for Option<T> {
    fn try_from_row_simple(column_name: &str, row: &rusqlite::Row) -> Result<Self, Error> {
        match T::try_from_row_simple(column_name, row) {
            Ok(it) => Ok(Some(it)),
            Err(_) => Ok(None),
        }
    }
}

impl<T: FromRow> FromRow for Option<T> {
    fn try_from_row(row: &rusqlite::Row, connection: &rusqlite::Connection) -> Result<Self, Error> {
        match T::try_from_row(row, connection) {
            Ok(it) => Ok(Some(it)),
            Err(_) => Ok(None),
        }
    }
}

pub trait ToTable<'a>: AsParams + AsColumns + FromRow {
    const NAME: &'static str;
    type Table: SqlTable<'a>;
}

impl<'a, T: ToTable<'a>> ToTable<'a> for Option<T> {
    const NAME: &'static str = T::NAME;

    type Table = T::Table;
}

// #[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
// pub enum SqlFailureBehavior {
//     #[default]
//     Abort,
//     Fail,
//     Ignore,
//     Replace,
//     Rollback,
// }

// impl Display for SqlFailureBehavior {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         match self {
//             SqlFailureBehavior::Abort => write!(f, "ABORT"),
//             SqlFailureBehavior::Fail => write!(f, "FAIL"),
//             SqlFailureBehavior::Ignore => write!(f, "IGNORE"),
//             SqlFailureBehavior::Replace => write!(f, "REPLACE"),
//             SqlFailureBehavior::Rollback => write!(f, "ROLLBACK"),
//         }
//     }
// }

pub trait SqlTable<'a>: Sized {
    type RowType: ToTable<'a>;
    type ValueType: partial::HasPartial;
    type FilterType: filter::Filter;
    // const INSERT_FAILURE_BEHAVIOR: SqlFailureBehavior;
    fn from_connection(connection: &'a Connection) -> Self;
    fn connection(&self) -> &'a Connection;

    fn insert(&self, row: Self::RowType) -> Result<bool, rusqlite::Error>;
    fn load_where(
        &self,
        filter: impl Into<Self::FilterType>,
    ) -> Result<Vec<Self::RowType>, rusqlite::Error>;
    fn update(
        &self,
        filter: impl Into<Self::FilterType>,
        updated: <Self::ValueType as partial::HasPartial>::Partial,
    ) -> Result<usize, rusqlite::Error>;
    // fn count(
    //     &self,
    //     filter: <Self::RowType as HasFilter>::Filter,
    // ) -> Result<usize, rusqlite::Error> {
    //     Ok(self.filter(filter)?.len())
    // }
    // fn migrate(&self, actual_columns: &[SqlColumn]) -> Result<(), rusqlite::Error>;
    // fn drain(
    //     &self,
    //     mut callback: impl FnMut(&Self::ValueType) -> bool,
    // ) -> Result<Vec<Self::ValueType>, rusqlite::Error> {
    //     Ok(self
    //         .filter(Default::default())?
    //         .into_iter()
    //         .filter_map(|r| {
    //             if (callback)(&r) {
    //                 let filter = <Self::ValueType as MustBeEqual<
    //                     <Self::RowType as HasFilter>::Filter,
    //                 >>::must_be_equal(&r);
    //                 self.delete(filter).ok()?;
    //                 Some(r)
    //             } else {
    //                 None
    //             }
    //         })
    //         .collect::<Vec<_>>())
    // }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OrderingAscDesc {
    Ascending,
    Descending,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OrderingNulls {
    NullsFirst,
    NullsLast,
}

#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub struct Ordering {
    pub asc_desc: Option<OrderingAscDesc>,
    pub nulls: Option<OrderingNulls>,
}

// #[derive(Debug, Default)]
// pub struct GenericOrder {
//     pub columns: Vec<(&'static str, Ordering)>,
// }

// impl GenericOrder {
//     fn to_sql(&self) -> String {
//         if self.columns.is_empty() {
//             return String::new();
//         }
//         let mut result: String = "ORDER BY".into();
//         for (i, (column, ordering)) in self.columns.iter().enumerate() {
//             if i > 0 {
//                 result.push(',');
//             }
//             result.push(' ');
//             result.push_str(column);
//             match ordering.asc_desc {
//                 Some(OrderingAscDesc::Ascending) => {
//                     result.push(' ');
//                     result.push_str("ASC");
//                 }
//                 Some(OrderingAscDesc::Descending) => {
//                     result.push(' ');
//                     result.push_str("DESC");
//                 }
//                 None => {}
//             }
//             match ordering.nulls {
//                 Some(OrderingNulls::NullsFirst) => {
//                     result.push(' ');
//                     result.push_str("NULLS FIRST");
//                 }
//                 Some(OrderingNulls::NullsLast) => {
//                     result.push(' ');
//                     result.push_str("NULLS LAST");
//                 }
//                 None => {}
//             }
//         }
//         result
//     }
// }

// impl GenericOrder {
//     pub fn add(&mut self, column: &'static str, order: Ordering) {
//         self.columns.push((column, order));
//     }
// }

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SqlColumn {
    pub name: Cow<'static, str>,
    pub r#type: SqlColumnType,
    pub is_primary: bool,
    pub is_unique: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SqlColumnType {
    Float,
    Integer,
    Null,
    Text,
    Blob,
    OptionalFloat,
    OptionalInteger,
    OptionalText,
    OptionalBlob,
}

impl SqlColumnType {
    pub fn as_sql(&self) -> &'static str {
        match self {
            // TODO: Find good handling for optional values!
            // SqlColumnType::Float => "REAL NOT NULL",
            // SqlColumnType::Integer => "INTEGER NOT NULL",
            // SqlColumnType::Text => "TEXT NOT NULL",
            // SqlColumnType::Blob => "BLOB NOT NULL",
            SqlColumnType::OptionalFloat | Self::Float => "REAL",
            SqlColumnType::OptionalInteger | Self::Integer => "INTEGER",
            SqlColumnType::OptionalText | Self::Text => "TEXT",
            SqlColumnType::OptionalBlob | Self::Blob => "BLOB",
            SqlColumnType::Null => "NULL",
        }
    }

    const fn to_optional(this: SqlColumnType) -> SqlColumnType {
        match this {
            SqlColumnType::OptionalFloat | SqlColumnType::Float => Self::OptionalFloat,
            SqlColumnType::OptionalInteger | SqlColumnType::Integer => Self::OptionalInteger,
            SqlColumnType::Null => Self::Null,
            SqlColumnType::OptionalText | SqlColumnType::Text => Self::OptionalText,
            SqlColumnType::OptionalBlob | SqlColumnType::Blob => Self::OptionalBlob,
        }
    }
}

pub fn insert_into_table<'a, T: ToTable<'a> + Clone>(
    connection: &&'a rusqlite::Connection,
    value: T,
) -> Result<bool, rusqlite::Error> {
    let columns = T::columns(None, false, false)
        .into_iter()
        .map(|c| c.name)
        .fold(String::new(), |mut acc, cur| {
            if acc.is_empty() {
                format!("\"{cur}\"")
            } else {
                acc.push_str(", ");
                acc.push('"');
                acc.push_str(&cur);
                acc.push('"');
                acc
            }
        });
    let values = (0..T::COLUMN_COUNT)
        .map(|v| v + 1)
        .fold(String::new(), |mut acc, cur| {
            if acc.is_empty() {
                format!("?{cur}")
            } else {
                acc.push_str(", ?");
                acc.push_str(&cur.to_string());
                acc
            }
        });

    let sql = format!("INSERT INTO {} ({columns}) VALUES ({values})", T::NAME,);
    debug_sql(&sql);

    let mut stmt = connection.prepare(&sql)?;
    let params = value.as_params();
    let params: Vec<_> = params.iter().map(|p| p.as_dyn()).collect();
    match stmt.execute(params.as_slice()) {
        Ok(_) => return Ok(true),
        Err(rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error {
                code: ErrorCode::ConstraintViolation,
                ..
            },
            _,
        )) => return Ok(false),
        Err(e) => return Err(e),
    }
}

pub fn load_where<'a, T: ToTable<'a>, F: filter::Filter>(
    connection: &&'a rusqlite::Connection,
    filter: impl Into<F>,
) -> Result<Vec<T>, rusqlite::Error> {
    let mut sql = format!("SELECT * FROM {} WHERE ", T::NAME);
    let filter = filter.into();
    filter.to_sql(&mut sql, None);
    let sql = sql.trim_end_matches(" WHERE ");
    debug_sql(sql);
    let mut s = connection.prepare(sql)?;
    // TODO: Filters encode their params directly. We might wanna change that,
    // but for now, this is not needed.

    // let params = filter.as_params();
    // let params: Vec<_> = params.iter().map(|p| p.as_dyn()).collect();

    s.query(())?
        .mapped(|r| T::try_from_row(r, connection).map_err(|_| todo!()))
        .collect()
}

pub fn update<'a, T: ToTable<'a>, V: AsParamsOptional + AsColumnsOptional, F: filter::Filter>(
    connection: &&'a rusqlite::Connection,
    filter: impl Into<F>,
    value: V,
) -> Result<usize, rusqlite::Error> {
    let filter = filter.into();
    let columns = value
        .columns_skip_optional(None, false, false)
        .into_iter()
        .enumerate()
        .map(|(i, c)| format!("{} = ?{}", c.name, i + 1))
        .fold(String::new(), |mut acc: String, cur| {
            if acc.is_empty() {
                cur
            } else {
                acc.push_str(", ");
                acc.push_str(&cur);
                acc
            }
        });
    let mut sql = format!("UPDATE {} SET {columns}", T::NAME);
    sql.push_str(" WHERE ");
    filter.to_sql(&mut sql, None);
    let sql = sql.trim_end_matches(" WHERE ");
    debug_sql(sql);

    let mut statement = connection.prepare(&sql)?;
    let params = value.as_params_skip_optional();
    let params: Vec<_> = params.iter().map(|p| p.as_dyn()).collect();
    statement.execute(params.as_slice())
}
