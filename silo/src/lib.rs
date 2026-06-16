use std::{
    borrow::Cow,
    fmt::Debug,
    path::Path,
    sync::atomic::{AtomicBool, Ordering::SeqCst},
};

use chrono::{DateTime, Utc};
pub use rusqlite;
use rusqlite::{Connection, ErrorCode, Params, types::Null};

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

pub trait AsColumns: AsColumnsDynamicallySized {
    const COLUMN_COUNT: usize;

    // fn columns(parent: Option<&str>, is_unique: bool, is_primary: bool) -> Vec<SqlColumn>;
}

pub trait AsColumnsDynamicallySized {
    fn columns(parent: Option<&str>, is_unique: bool, is_primary: bool) -> Vec<SqlColumn>;
}

pub trait AsParams {
    fn as_params<'b>(&'b self) -> Vec<ToSqlDyn<'b>>;
}

pub trait AsColumnsOptional {
    fn columns_skip_optional(
        &self,
        parent: Option<&str>,
        is_unique: bool,
        is_primary: bool,
    ) -> Vec<SqlColumn>;
}

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
        impl<'a> HasPartial for $t {
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
// impl_as_params!(&'a str, SqlColumnType::Text);

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Rusqlite(#[from] rusqlite::Error),
    #[error("No column named {0} could be found.")]
    MissingColumn(Cow<'static, str>),
    #[error("Value has type {1}, which could not be converted to {0}.")]
    WrongColumnType(Cow<'static, str>, rusqlite::types::Type),
    #[error("Could not migrate value because of this: {0}.")]
    CouldNotMigrate(Cow<'static, str>),
    #[error("Todo: {0}")]
    Todo(String),
    #[error("IllFormattedColumn: {1} cannot be parsed into {0}: {2:?}")]
    IllFormattedColumn(
        Cow<'static, str>,
        String,
        Option<Box<dyn std::error::Error>>,
    ),
}

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

pub trait PartialType<T> {
    fn transpose(self) -> Option<T>;
}

impl<T> PartialType<T> for Option<T> {
    fn transpose(self) -> Option<T> {
        self
    }
}

pub trait HasPartial<T = Self>: Sized + Into<Self::Partial> {
    // TODO: find out why we do not have partial type here!
    type Partial: Default;
    // type Partial: PartialType<T>;
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

    // fn columns() -> Vec<SqlColumn> {
    //     let mut result = Vec::with_capacity(Self::COLUMN_COUNT);
    //     Self::fill_columns(&mut result);
    //     result
    // }

    // fn fill_columns(columns: &mut Vec<SqlColumn>);
}

// #[diagnostic::on_unimplemented(
//     message = "ToColumns is not implemented for `{Self}`",
//     label = "My Label",
//     note = "You can either add an primary_key or derive ToColumns.",
//     note = "Read documentation on ToColumns for more information."
// )]
// pub trait ToColumns: AsParams + AsColumns + FromRow {
//     fn columns() -> Vec<SqlColumn> {
//         let mut result = Vec::with_capacity(Self::COLUMN_COUNT);
//         Self::fill_columns(&mut result);
//         result
//     }

//     fn fill_columns(columns: &mut Vec<SqlColumn>);
// }

impl<'a, T: ToTable<'a>> ToTable<'a> for Option<T>
// where
//     Option<<T as HasPartialRepresentation>::Partial>: From<Option<T>>,
{
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
    type ValueType: HasPartial;
    type FilterType: filter::Filter;
    // const INSERT_FAILURE_BEHAVIOR: SqlFailureBehavior;
    fn from_connection(connection: &'a Connection) -> Self;
    fn connection(&self) -> &'a Connection;

    fn insert(&self, row: Self::RowType) -> Result<bool, rusqlite::Error>;
    fn load_where(&self, filter: Self::FilterType) -> Result<Vec<Self::RowType>, rusqlite::Error>;
    fn update(
        &self,
        filter: Self::FilterType,
        updated: <Self::ValueType as HasPartial>::Partial,
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

// #[derive(Debug, Default)]
// pub struct GenericFilter {
//     pub columns: HashMap<Cow<'static, str>, SqlColumnFilter<SqlValue>>,
// }

// impl GenericFilter {
//     pub fn insert(
//         &mut self,
//         name: Cow<'static, str>,
//         value: impl IntoSqlColumnFilter,
//         string_storage: &mut StaticStringStorage,
//     ) {
//         Self::insert_into_columns(name, &mut self.columns, value, string_storage);
//     }

//     pub fn insert_into_columns(
//         name: Cow<'static, str>,
//         columns: &mut HashMap<Cow<'static, str>, SqlColumnFilter<SqlValue>>,
//         value: impl IntoSqlColumnFilter,
//         string_storage: &mut StaticStringStorage,
//     ) {
//         let values = value.into_sql_column_filter(name, string_storage);
//         for (name, value) in values {
//             columns.insert(name, value.clone());
//         }
//     }

//     fn get_params(&self) -> () {
//         ()
//     }

//     fn to_sql(&self) -> String {
//         use std::fmt::Write;
//         if !self
//             .columns
//             .iter()
//             .any(|c| !matches!(c.1, SqlColumnFilter::Ignored))
//         {
//             return String::new();
//         }
//         let mut result: String = "WHERE".into();
//         let mut emitted = false;
//         for (name, filter) in &self.columns {
//             if matches!(filter, SqlColumnFilter::Ignored) {
//                 continue;
//             }
//             if emitted {
//                 write!(result, " AND").expect("Infallibe");
//             }
//             write!(result, " {name} {}", filter.to_sql()).expect("Infallible");
//             emitted = true;
//         }
//         result
//     }
// }

// pub struct PrimaryKey;

#[derive(Debug, Clone)]
pub enum SqlValue {
    Float(f64),
    Integer(i64),
    Null,
    Text(String),
    Blob(Vec<u8>),
}

impl Into<SqlValue> for f64 {
    fn into(self) -> SqlValue {
        SqlValue::Float(self)
    }
}

impl Into<SqlValue> for f32 {
    fn into(self) -> SqlValue {
        SqlValue::Float(self as f64)
    }
}

macro_rules! into_sql_value_integer {
    ($t:ty) => {
        impl Into<SqlValue> for $t {
            fn into(self) -> SqlValue {
                SqlValue::Integer(self as i64)
            }
        }
    };
}

into_sql_value_integer!(bool);
into_sql_value_integer!(i8);
into_sql_value_integer!(i16);
into_sql_value_integer!(i32);
into_sql_value_integer!(i64);
into_sql_value_integer!(isize);
into_sql_value_integer!(u8);
into_sql_value_integer!(u16);
into_sql_value_integer!(u32);
into_sql_value_integer!(usize);
into_sql_value_integer!(u64);

impl Into<SqlValue> for String {
    fn into(self) -> SqlValue {
        SqlValue::Text(self)
    }
}

impl Into<SqlValue> for &str {
    fn into(self) -> SqlValue {
        SqlValue::Text(self.into())
    }
}

impl Into<SqlValue> for Time {
    fn into(self) -> SqlValue {
        const TIME_FORMAT: &[FormatItem<'_>] = format_description!(
            version = 2,
            "[hour]:[minute][optional [:[second][optional [.[subsecond]]]]]"
        );
        SqlValue::Text(self.format(&TIME_FORMAT).unwrap())
    }
}

impl Into<SqlValue> for Date {
    fn into(self) -> SqlValue {
        const DATE_FORMAT: &[FormatItem<'_>] =
            format_description!(version = 2, "[year]-[month]-[day]");
        SqlValue::Text(self.format(&DATE_FORMAT).unwrap())
    }
}

impl Into<SqlValue> for OffsetDateTime {
    fn into(self) -> SqlValue {
        const OFFSET_DATE_TIME_ENCODING: &[FormatItem<'_>] = format_description!(
            version = 2,
            "[year]-[month]-[day] [hour]:[minute]:[second].[subsecond][offset_hour sign:mandatory]:[offset_minute]"
        );
        SqlValue::Text(self.format(&OFFSET_DATE_TIME_ENCODING).unwrap())
    }
}

impl SqlValue {
    fn to_sql(&self) -> String {
        match self {
            SqlValue::Float(it) => it.to_string(),
            SqlValue::Integer(it) => it.to_string(),
            SqlValue::Null => "NULL".to_string(),
            SqlValue::Text(it) => {
                format!("'{}'", it.replace('\'', "''"))
            }
            SqlValue::Blob(_items) => todo!(),
        }
    }
}

// #[derive(Debug, Clone, PartialEq, Eq)]
// pub enum SqlColumnDefinition {
//     Prefixed {
//         prefix: &'static str,
//         children: &'static [SqlColumnDefinition],
//     },
//     Column(SqlColumn),
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

// pub trait PartialRow {
//     fn used_column_names(&self, column_name: Option<String>) -> Vec<String>;
//     fn used_values(&self) -> Vec<&dyn rusqlite::ToSql>;
// }

// impl<T: AsParams> PartialRow for Option<T> {
//     fn used_column_names(&self, column_name: Option<String>) -> Vec<String> {
//         if self.is_some() {
//             vec![column_name.expect("Needs column name!")]
//         } else {
//             Vec::new()
//         }
//     }

//     fn used_values(&self) -> Vec<&dyn rusqlite::ToSql> {
//         if let Some(value) = self {
//             value.as_params()
//         } else {
//             Vec::new()
//         }
//     }
// }

// impl<T: AsParams> PartialRow for Vec<T> {
//     fn used_column_names(&self, column_name: Option<String>) -> Vec<String> {
//         if self.has_values() {
//             vec![column_name.expect("Needs column name!")]
//         } else {
//             Vec::new()
//         }
//     }

//     fn used_values(&self) -> Vec<&dyn rusqlite::ToSql> {
//         if self.len() > 1 {
//             eprintln!("Ahh, this is lossy!");
//         }
//         if let Some(value) = self.into_iter().next() {
//             value.as_params()
//         } else {
//             Vec::new()
//         }
//     }
// }

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

pub fn load_where<'a, T: ToTable<'a>>(
    connection: &&'a rusqlite::Connection,
    filter: impl filter::Filter,
) -> Result<Vec<T>, rusqlite::Error> {
    let mut sql = format!("SELECT * FROM {} WHERE ", T::NAME);
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

pub fn update<'a, T: ToTable<'a>, V: AsParamsOptional + AsColumnsOptional>(
    connection: &&'a rusqlite::Connection,
    filter: impl filter::Filter,
    value: V,
) -> Result<usize, rusqlite::Error> {
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
