use std::{
    borrow::Cow,
    fmt::{Debug, Display},
    path::Path,
};

use chrono::{DateTime, Utc};
pub use rusqlite;
use rusqlite::{Connection, types::Null};

pub use konst;
pub mod derive {
    pub use silo_derive::ToTable;
}

use time::macros::format_description;
use time::{Date, Time};
use time::{OffsetDateTime, format_description::FormatItem};

use crate::filter::ToFilter;

#[cfg(feature = "debug_sql")]
pub const ENABLE_DEBUG_PRINTING: bool = true;
#[cfg(not(feature = "debug_sql"))]
pub const ENABLE_DEBUG_PRINTING: bool = false;

pub mod filter;

#[cfg(test)]
mod test {
    #[test]
    fn test() -> Result<(), Box<dyn Error>> {
        let db = Database::create_in_memory()?;
        let coords = db.load::<Coord>()?;
        coords.insert(Coord {
            x: 430.0,
            y: 13324.3124,
        })?;
        // let coords = coords.filter(CoordFilter::default().y_should_be(13324.3124))?;
        // assert_eq!(coords.as_slice(), &[]);
        Ok(())
    }
    use std::{borrow::Cow, error::Error};

    use rusqlite::{Connection, OptionalExtension};

    use crate::{
        Database, FromRow, HasPartial, MigrationHandler, PartialType, SqlColumn, SqlColumnType,
        SqlTable, TableAsParams, ToTable,
        filter::{GenericFilter, ToFilter},
    };

    #[derive(Debug, PartialEq)]
    struct Coord {
        x: f64,
        y: f64,
    }

    #[derive(Debug, PartialEq, Default)]
    struct PartialCoord {
        x: Option<f64>,
        y: Option<f64>,
    }

    impl PartialType<Coord> for PartialCoord {
        fn transpose(self) -> Option<Coord> {
            let x = self.x?;
            let y = self.y?;
            Some(Coord { x, y })
        }
    }

    impl FromRow for PartialCoord {
        fn try_from_row(
            _row: &rusqlite::Row,
            _connection: &rusqlite::Connection,
        ) -> Result<Self, super::Error> {
            // row.get("x")
            todo!();
        }
    }

    impl From<Coord> for PartialCoord {
        fn from(value: Coord) -> Self {
            Self {
                x: value.x.into(),
                y: value.y.into(),
            }
        }
    }

    impl FromRow for Coord {
        fn try_from_row(
            row: &rusqlite::Row,
            _connection: &rusqlite::Connection,
        ) -> Result<Self, super::Error> {
            let x: f64 = row.get("x").optional().unwrap().unwrap();
            let y: f64 = row.get("y").optional().unwrap().unwrap();
            Ok(Self { x, y })
        }
    }

    impl TableAsParams for Coord {
        const COLUMN_COUNT: usize = 2;
        fn as_params<'b>(&'b self) -> Vec<&'b dyn rusqlite::ToSql> {
            vec![&self.x, &self.y]
        }
    }

    impl MigrationHandler for Coord {}

    impl HasPartial for Coord {
        type Partial = PartialCoord;
    }

    impl<'a> ToTable<'a> for Coord {
        type Table = CoordTable<'a>;

        const NAME: &'static str = "CoordTable";

        fn fill_columns(columns: &mut Vec<SqlColumn>) {
            columns.push(SqlColumn {
                name: Cow::Borrowed("x"),
                r#type: SqlColumnType::Float,
                is_primary: false,
                is_unique: false,
            });
            columns.push(SqlColumn {
                name: Cow::Borrowed("y"),
                r#type: SqlColumnType::Float,
                is_primary: false,
                is_unique: false,
            });
        }
    }

    struct CoordTable<'a> {
        connection: &'a Connection,
    }

    #[derive(Default)]
    struct CoordFilter {
        generic: GenericFilter,
    }

    impl ToFilter for CoordFilter {
        fn to_filter(self) -> GenericFilter {
            self.generic
        }
    }

    impl<'a> SqlTable<'a> for CoordTable<'a> {
        type RowType = Coord;
        type ValueType = Coord;

        fn insert(&self, row: Self::RowType) -> Result<(), rusqlite::Error> {
            let columns = Self::RowType::columns().into_iter().map(|c| c.name).fold(
                String::new(),
                |mut acc, cur| {
                    if acc.is_empty() {
                        cur.clone().into_owned()
                    } else {
                        acc.push_str(", ");
                        acc.push_str(&cur);
                        acc
                    }
                },
            );
            let values = (0..Self::RowType::COLUMN_COUNT).map(|v| v + 1).fold(
                String::new(),
                |mut acc, cur| {
                    if acc.is_empty() {
                        format!("?{cur}")
                    } else {
                        acc.push_str(", ?");
                        acc.push_str(&cur.to_string());
                        acc
                    }
                },
            );

            let mut stmt = self.connection.prepare(&format!(
                "INSERT OR IGNORE INTO {} ({columns}) VALUES ({values})",
                Self::RowType::NAME
            ))?;
            stmt.execute(row.as_params().as_slice())?;
            Ok(())
        }

        // fn filter(&self, filter: CoordFilter) -> Result<Vec<Coord>, rusqlite::Error> {
        //     let generic = filter.into_generic(&mut self.string_storage.lock().unwrap(), None);
        //     crate::query_table_filtered::<Self::RowType, Self::ValueType>(
        //         &self.connection,
        //         &mut self.string_storage.lock().unwrap(),
        //         generic,
        //         GenericOrder::default(),
        //     )
        // }

        // fn delete(&self, filter: CoordFilter) -> Result<usize, rusqlite::Error> {
        //     let generic = filter.into_generic(&mut self.string_storage.lock().unwrap(), None);
        //     crate::delete_table_filtered::<Self::RowType>(&self.connection, generic)
        // }

        fn from_connection(connection: &'a Connection) -> Self {
            Self { connection }
        }
        type FilterType = CoordFilter;

        fn connection(&self) -> &'a Connection {
            self.connection
        }
        const INSERT_FAILURE_BEHAVIOR: crate::SqlFailureBehavior =
            crate::SqlFailureBehavior::Ignore;

        // fn update(
        //     &self,
        //     filter: CoordFilter,
        //     updated: PartialCoord,
        // ) -> Result<(), rusqlite::Error> {
        //     Err(rusqlite::Error::InvalidQuery)
        // }

        // fn migrate(&self, actual_columns: &[SqlColumn]) -> Result<(), rusqlite::Error> {
        //     handle_migration::<Self::RowType>(
        //         self.connection,
        //         &mut self.string_storage.lock().unwrap(),
        //         actual_columns,
        //     )
        // }
    }
}

pub trait EnumHelper {
    fn variant(&self) -> &'static str {
        *self.variant_ref()
    }

    fn variant_ref(&self) -> &'static &'static str;
}

pub fn handle_migration<T: for<'a> ToTable<'a> + MigrationHandler>(
    connection: &rusqlite::Connection,
    actual_columns: &[SqlColumn],
) -> Result<(), rusqlite::Error>
where
    <T as HasPartial>::Partial: PartialType<T> + FromRow,
{
    let mut sql = "CREATE TABLE temporary".to_string();
    sql.push_str(" (");
    for (i, column) in T::columns().into_iter().enumerate() {
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
    #[cfg(feature = "debug_sql")]
    dbg!(&sql);
    connection.execute(&sql, ())?;
    let sql = format!("SELECT * FROM {}", T::NAME);
    #[cfg(feature = "debug_sql")]
    dbg!(&sql);
    let mut s = connection.prepare(&sql)?;
    let entries: Vec<_> = s
        .query_map((), |row| {
            let partial = <T as HasPartial>::Partial::try_from_row(row, connection).unwrap();
            Ok(<T as MigrationHandler>::migrate(partial, row, connection))
        })?
        .filter_map(|x| x.ok().transpose().ok().flatten())
        .collect();

    let columns = T::columns()
        .into_iter()
        .map(|c| c.name)
        .fold(String::new(), |mut acc, cur| {
            if acc.is_empty() {
                cur.clone().into_owned()
            } else {
                acc.push_str(", ");
                acc.push_str(&cur);
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

    let sql = format!("INSERT OR IGNORE INTO temporary ({columns}) VALUES ({values})",);
    #[cfg(feature = "debug_sql")]
    dbg!(&sql);
    let mut s = connection.prepare(&sql)?;

    for entry in entries {
        s.execute(entry.as_params().as_slice())?;
    }

    let sql = format!("DROP TABLE {}", T::NAME);
    #[cfg(feature = "debug_sql")]
    dbg!(&sql);
    connection.execute(&sql, ())?;
    let sql = format!("ALTER TABLE temporary RENAME TO {}", T::NAME);
    #[cfg(feature = "debug_sql")]
    dbg!(&sql);
    connection.execute(&sql, ())?;
    Ok(())
}

pub struct Database {
    connection: rusqlite::Connection,
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
        connection.execute("DROP TABLE IF EXISTS temporary", ())?;
        Ok(Self::new_from_connection(connection))
    }

    pub fn open(path: impl AsRef<Path>) -> Result<Self, rusqlite::Error> {
        let connection = rusqlite::Connection::open(path)?;
        connection.execute("DROP TABLE IF EXISTS temporary", ())?;
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
        for (i, column) in T::columns().into_iter().enumerate() {
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
        #[cfg(feature = "debug_sql")]
        dbg!(&sql);

        self.connection.execute(&sql, ())?;
        Ok(())
    }

    pub fn check<'a, T: ToTable<'a>>(&'a self) -> Result<(), rusqlite::Error> {
        let mut columns = vec![];
        #[cfg(feature = "debug_sql")]
        {
            let sql = format!("PRAGMA table_info({})", T::NAME);
            dbg!(&sql);
        }
        self.connection.pragma(None, "table_info", T::NAME, |row| {
            let name: String = row.get("name")?;
            let type_name: String = row.get("type")?;
            let is_not_null: bool = row.get("notnull")?;
            let r#type = match (type_name.as_str(), !is_not_null) {
                ("TEXT", false) => SqlColumnType::Text,
                ("TEXT", true) => SqlColumnType::OptionalText,
                ("INTEGER", false) => SqlColumnType::Integer,
                ("INTEGER", true) => SqlColumnType::OptionalInteger,
                ("REAL", false) => SqlColumnType::Float,
                ("REAL", true) => SqlColumnType::OptionalFloat,
                ("BLOB", false) => SqlColumnType::Blob,
                ("BLOB", true) => SqlColumnType::OptionalBlob,
                ("NULL", _) => SqlColumnType::Null,
                unknown => todo!("Unknown: {unknown:#?}"),
            };
            let is_primary: bool = row.get("pk")?;
            // let name = self
            //     .static_string_storage
            //     .lock()
            //     .unwrap()
            //     .store_heaped(name);
            columns.push(SqlColumn {
                name: name.into(),
                r#type,
                is_primary,
                is_unique: false,
            });
            Ok(())
        })?;
        let mut unique_indices = vec![];
        #[cfg(feature = "debug_sql")]
        {
            let sql = format!("PRAGMA index_list({})", T::NAME);
            dbg!(&sql);
        }
        self.connection.pragma(None, "index_list", T::NAME, |row| {
            if row.get::<&str, bool>("unique")? {
                unique_indices.push(row.get::<&str, String>("name")?);
            }
            Ok(())
        })?;
        for index in unique_indices {
            #[cfg(feature = "debug_sql")]
            {
                let sql = format!("PRAGMA index_info({})", &index);
                dbg!(&sql);
            }
            self.connection.pragma(None, "index_info", index, |row| {
                let idx: usize = row.get("cid")?;
                columns[idx].is_unique = true;
                Ok(())
            })?;
        }
        if compare_columns(&columns, &T::columns()) {
            let table = self.load::<T>()?;
            todo!()
            // table.migrate(&columns)?;
        }

        Ok(())
        // self.connection.pragma_query(schema_name, "table_info", f)
    }
}

#[derive(Debug)]
#[allow(dead_code)]
enum TableAlteration {
    InsertColumn(SqlColumn),
    ChangeType(usize, SqlColumnType),
    ChangeIsPrimary(usize, bool),
    ChangeIsUnique(usize, bool),
    DeleteColumn(Cow<'static, str>),
}

fn compare_columns(actual: &[SqlColumn], expected: &[SqlColumn]) -> bool {
    #[cfg(feature = "debug_sql")]
    dbg!(actual, expected);
    fn find_by_name<'a>(columns: &'a [SqlColumn], name: &str) -> Option<&'a SqlColumn> {
        columns.iter().find(|c| c.name == name)
    }
    let mut necessary_alternations = Vec::new();
    let mut seen_columns = Vec::new();
    for (index, column) in expected.into_iter().enumerate() {
        seen_columns.push(&column.name);
        match find_by_name(actual, &column.name) {
            Some(actual) => {
                if actual.is_primary != column.is_primary {
                    necessary_alternations
                        .push(TableAlteration::ChangeIsPrimary(index, column.is_primary));
                }
                if actual.is_unique != column.is_unique {
                    necessary_alternations
                        .push(TableAlteration::ChangeIsUnique(index, column.is_unique));
                }
                if actual.r#type != SqlColumnType::to_optional(column.r#type) {
                    necessary_alternations.push(TableAlteration::ChangeType(index, column.r#type));
                }
            }
            None => {
                necessary_alternations.push(TableAlteration::InsertColumn(column.clone()));
            }
        }
    }
    for column in actual {
        if !seen_columns.contains(&&column.name) {
            necessary_alternations.push(TableAlteration::DeleteColumn(column.name.clone()));
        }
    }
    return !necessary_alternations.is_empty();
}

pub trait AsColumns {
    const COLUMN_COUNT: usize;

    fn columns(parent: Option<&str>) -> Vec<SqlColumn>;
}

pub trait AsParams: AsColumns {
    fn as_params<'b>(&'b self) -> Vec<&'b dyn rusqlite::ToSql>;
}

pub trait TableAsParams {
    const COLUMN_COUNT: usize;

    fn as_params<'b>(&'b self) -> Vec<&'b dyn rusqlite::ToSql>;
}

impl<T: AsParams + AsColumns> AsParams for Option<T> {
    fn as_params<'b>(&'b self) -> Vec<&'b dyn rusqlite::ToSql> {
        match self {
            Some(it) => it.as_params(),
            None => vec![&Null; T::COLUMN_COUNT],
        }
    }
}

impl<T: AsColumns> AsColumns for Option<T> {
    const COLUMN_COUNT: usize = T::COLUMN_COUNT;

    fn columns(parent: Option<&str>) -> Vec<SqlColumn> {
        T::columns(parent)
    }
}

impl<T: TableAsParams> TableAsParams for Option<T> {
    const COLUMN_COUNT: usize = T::COLUMN_COUNT;
    fn as_params<'b>(&'b self) -> Vec<&'b dyn rusqlite::ToSql> {
        match self {
            Some(it) => it.as_params(),
            None => vec![&Null; T::COLUMN_COUNT],
        }
    }
}

pub trait IsSingleColumn {
    const SQL_COLUMN_TYPE: SqlColumnType;
}

impl<T: IsSingleColumn> AsColumns for T {
    const COLUMN_COUNT: usize = 1;

    fn columns(parent: Option<&str>) -> Vec<SqlColumn> {
        vec![SqlColumn {
            name: parent.unwrap().to_string().into(),
            r#type: T::SQL_COLUMN_TYPE,
            is_primary: false,
            is_unique: false,
        }]
    }
}

macro_rules! impl_as_params {
    ($t:ty, $column_type:expr) => {
        impl HasPartial for $t {
            type Partial = Option<$t>;
        }

        impl IsSingleColumn for $t {
            const SQL_COLUMN_TYPE: SqlColumnType = $column_type;
        }

        impl AsParams for $t {
            fn as_params<'b>(&'b self) -> Vec<&'b dyn rusqlite::ToSql> {
                vec![self]
            }
        }

        impl ExtractFromRow for $t {
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

        impl AsForeignReference for $t {
            fn insert_as_foreign_reference(
                self,
                _: &rusqlite::Connection,
            ) -> Result<(), rusqlite::Error> {
                Ok(())
            }
        }
    };
}

macro_rules! impl_as_params_and_nan_is_none {
    ($t:ty, $column_type:expr) => {
        impl HasPartial for $t {
            type Partial = Option<$t>;
        }

        impl IsSingleColumn for $t {
            const SQL_COLUMN_TYPE: SqlColumnType = $column_type;
        }

        impl AsParams for $t {
            fn as_params<'b>(&'b self) -> Vec<&'b dyn rusqlite::ToSql> {
                vec![self]
            }
        }

        impl ExtractFromRow for $t {
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

        impl AsForeignReference for $t {
            fn insert_as_foreign_reference(
                self,
                _: &rusqlite::Connection,
            ) -> Result<(), rusqlite::Error> {
                Ok(())
            }
        }
    };
}

macro_rules! impl_as_params_and_column_filter {
    ($t:ty, $column_type:expr) => {
        impl<'a> HasPartial for $t {
            type Partial = Option<$t>;
        }

        impl<'a> IsSingleColumn for $t {
            const SQL_COLUMN_TYPE: SqlColumnType = $column_type;
        }

        impl<'a> AsParams for $t {
            fn as_params<'b>(&'b self) -> Vec<&'b dyn rusqlite::ToSql> {
                vec![self]
            }
        }

        impl<'a> AsForeignReference for $t {
            fn insert_as_foreign_reference(
                self,
                _: &rusqlite::Connection,
            ) -> Result<(), rusqlite::Error> {
                Ok(())
            }
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
impl_as_params!(OffsetDateTime, SqlColumnType::Text);
impl_as_params_and_nan_is_none!(f32, SqlColumnType::Float);
impl_as_params_and_nan_is_none!(f64, SqlColumnType::Float);
impl_as_params!(String, SqlColumnType::Text);
impl_as_params_and_column_filter!(&'a str, SqlColumnType::Text);

// macro_rules! related_sql_column_type {
//     ($v:path, $t:ty) => {
//         impl HasSqlColumnType for $t {
//             const SQL_COLUMN_TYPE: SqlColumnType = $v;
//         }
//     };
// }

// related_sql_column_type!(SqlColumnType::Integer, bool);
// related_sql_column_type!(SqlColumnType::Integer, i8);
// related_sql_column_type!(SqlColumnType::Integer, i16);
// related_sql_column_type!(SqlColumnType::Integer, i32);
// related_sql_column_type!(SqlColumnType::Integer, i64);
// related_sql_column_type!(SqlColumnType::Integer, isize);
// related_sql_column_type!(SqlColumnType::Integer, u8);
// related_sql_column_type!(SqlColumnType::Integer, u16);
// related_sql_column_type!(SqlColumnType::Integer, u32);
// related_sql_column_type!(SqlColumnType::Integer, u64);
// related_sql_column_type!(SqlColumnType::Integer, usize);
// related_sql_column_type!(SqlColumnType::Float, f32);
// related_sql_column_type!(SqlColumnType::Float, f64);
// related_sql_column_type!(SqlColumnType::Text, String);
// related_sql_column_type!(SqlColumnType::Text, Time);
// related_sql_column_type!(SqlColumnType::Text, Date);
// related_sql_column_type!(SqlColumnType::Text, OffsetDateTime);
// related_sql_column_type!(SqlColumnType::Text, Datetime<T>);

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

pub trait MigrationHandler: Sized + HasPartial
where
    <Self as HasPartial>::Partial: PartialType<Self>,
{
    fn migrate(
        partial: Self::Partial,
        row: &rusqlite::Row,
        connection: &rusqlite::Connection,
    ) -> Result<Self, Error> {
        partial
            .transpose()
            .ok_or(Error::CouldNotMigrate("Missing values".into()))
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

pub trait ToTable<'a>: TableAsParams + FromRow {
    const NAME: &'static str;
    type Table: SqlTable<'a>;

    fn columns() -> Vec<SqlColumn> {
        let mut result = Vec::with_capacity(Self::COLUMN_COUNT);
        Self::fill_columns(&mut result);
        result
    }

    fn fill_columns(columns: &mut Vec<SqlColumn>);

    fn insert_foreign_references(
        self,
        connection: &rusqlite::Connection,
    ) -> Result<(), rusqlite::Error> {
        Ok(())
    }
}

pub trait AsForeignReference {
    fn insert_as_foreign_reference(
        self,
        connection: &rusqlite::Connection,
    ) -> Result<(), rusqlite::Error>;
}

#[diagnostic::on_unimplemented(
    message = "ToColumns is not implemented for `{Self}`",
    label = "My Label",
    note = "You can either add an primary_key or derive ToColumns.",
    note = "Read documentation on ToColumns for more information."
)]
pub trait ToColumns: AsParams + FromRow {
    fn columns() -> Vec<SqlColumn> {
        let mut result = Vec::with_capacity(Self::COLUMN_COUNT);
        Self::fill_columns(&mut result);
        result
    }

    fn fill_columns(columns: &mut Vec<SqlColumn>);
}

impl<'a, T: ToTable<'a>> ToTable<'a> for Option<T>
// where
//     Option<<T as HasPartialRepresentation>::Partial>: From<Option<T>>,
{
    const NAME: &'static str = T::NAME;

    type Table = T::Table;

    fn fill_columns(columns: &mut Vec<SqlColumn>) {
        T::fill_columns(columns);
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum SqlFailureBehavior {
    #[default]
    Abort,
    Fail,
    Ignore,
    Replace,
    Rollback,
}

impl Display for SqlFailureBehavior {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SqlFailureBehavior::Abort => write!(f, "ABORT"),
            SqlFailureBehavior::Fail => write!(f, "FAIL"),
            SqlFailureBehavior::Ignore => write!(f, "IGNORE"),
            SqlFailureBehavior::Replace => write!(f, "REPLACE"),
            SqlFailureBehavior::Rollback => write!(f, "ROLLBACK"),
        }
    }
}

pub trait SqlTable<'a>: Sized {
    type RowType: ToTable<'a>;
    type ValueType;
    type FilterType: filter::ToFilter;
    const INSERT_FAILURE_BEHAVIOR: SqlFailureBehavior;
    fn from_connection(connection: &'a Connection) -> Self;
    fn connection(&self) -> &'a Connection;
    // fn filter(
    //     &self,
    //     filter: <Self::RowType as HasFilter>::Filter,
    // ) -> Result<Vec<Self::ValueType>, rusqlite::Error>;
    // fn delete(
    //     &self,
    //     filter: <Self::RowType as HasFilter>::Filter,
    // ) -> Result<usize, rusqlite::Error>;

    fn insert(&self, row: Self::RowType) -> Result<(), rusqlite::Error>;
    fn load_where(
        &self,
        filter: impl FnOnce(Self::FilterType) -> Self::FilterType,
    ) -> Result<Vec<Self::RowType>, rusqlite::Error> {
        load_where::<Self>(
            self.connection(),
            filter(Self::FilterType::default()).to_filter(),
        )
    }
    // fn update(
    //     &self,
    //     filter: <Self::RowType as HasFilter>::Filter,
    //     updated: <Self::ValueType as HasPartial>::Partial,
    // ) -> Result<(), rusqlite::Error>;
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

// #[derive(Default, Clone, Debug)]
// pub enum SqlColumnFilter<T: Clone + std::fmt::Debug> {
//     #[default]
//     Ignored,
//     MustBeEqual(T),
//     Contains(T),
// }

// impl<T: Into<SqlValue> + Clone + std::fmt::Debug> SqlColumnFilter<T> {
//     pub fn into_generic(self) -> SqlColumnFilter<SqlValue> {
//         match self {
//             SqlColumnFilter::Ignored => SqlColumnFilter::Ignored,
//             SqlColumnFilter::MustBeEqual(it) => SqlColumnFilter::MustBeEqual(it.into()),
//             SqlColumnFilter::Contains(it) => SqlColumnFilter::Contains(it.into()),
//         }
//     }
// }

// impl SqlColumnFilter<SqlValue> {
//     pub fn to_sql(&self) -> String {
//         match self {
//             SqlColumnFilter::Ignored => unreachable!(),
//             SqlColumnFilter::MustBeEqual(v) => format!(" = {}", v.to_sql()),
//             SqlColumnFilter::Contains(v) => {
//                 let string_representation = v.to_sql();
//                 if string_representation.starts_with('\'') && string_representation.ends_with('\'')
//                 {
//                     format!(
//                         " LIKE '%{}%'",
//                         &string_representation[1..string_representation.len() - 1]
//                     )
//                 } else {
//                     // Fallback to must be equal
//                     format!(" = {}", v.to_sql())
//                 }
//             }
//         }
//     }
// }

// pub trait IntoSqlColumnFilter {
//     fn into_sql_column_filter(
//         self,
//         name: Cow<'static, str>,
//         string_storage: &mut StaticStringStorage,
//     ) -> Vec<(Cow<'static, str>, SqlColumnFilter<SqlValue>)>;
// }

// impl<T: IntoGenericFilter> IntoSqlColumnFilter for T {
//     fn into_sql_column_filter(
//         self,
//         name: Cow<'static, str>,
//         string_storage: &mut StaticStringStorage,
//     ) -> Vec<(Cow<'static, str>, SqlColumnFilter<SqlValue>)> {
//         let generic = self.into_generic(string_storage, Some(name));
//         generic.columns.into_iter().collect()
//     }
// }

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

#[derive(Debug, Default)]
pub struct GenericOrder {
    pub columns: Vec<(&'static str, Ordering)>,
}

impl GenericOrder {
    fn to_sql(&self) -> String {
        if self.columns.is_empty() {
            return String::new();
        }
        let mut result: String = "ORDER BY".into();
        for (i, (column, ordering)) in self.columns.iter().enumerate() {
            if i > 0 {
                result.push(',');
            }
            result.push(' ');
            result.push_str(column);
            match ordering.asc_desc {
                Some(OrderingAscDesc::Ascending) => {
                    result.push(' ');
                    result.push_str("ASC");
                }
                Some(OrderingAscDesc::Descending) => {
                    result.push(' ');
                    result.push_str("DESC");
                }
                None => {}
            }
            match ordering.nulls {
                Some(OrderingNulls::NullsFirst) => {
                    result.push(' ');
                    result.push_str("NULLS FIRST");
                }
                Some(OrderingNulls::NullsLast) => {
                    result.push(' ');
                    result.push_str("NULLS LAST");
                }
                None => {}
            }
        }
        result
    }
}

impl GenericOrder {
    pub fn add(&mut self, column: &'static str, order: Ordering) {
        self.columns.push((column, order));
    }
}

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

pub struct PrimaryKey;

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SqlColumnDefinition {
    Prefixed {
        prefix: &'static str,
        children: &'static [SqlColumnDefinition],
    },
    Column(SqlColumn),
}

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

pub trait PartialRow {
    fn used_column_names(&self, column_name: Option<String>) -> Vec<String>;
    fn used_values(&self) -> Vec<&dyn rusqlite::ToSql>;
}

impl<T: AsParams> PartialRow for Option<T> {
    fn used_column_names(&self, column_name: Option<String>) -> Vec<String> {
        if self.is_some() {
            vec![column_name.expect("Needs column name!")]
        } else {
            Vec::new()
        }
    }

    fn used_values(&self) -> Vec<&dyn rusqlite::ToSql> {
        if let Some(value) = self {
            value.as_params()
        } else {
            Vec::new()
        }
    }
}

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
    on_failure: SqlFailureBehavior,
) -> Result<(), rusqlite::Error> {
    let columns = T::columns()
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

    let sql = format!(
        "INSERT OR {on_failure} INTO {} ({columns}) VALUES ({values})",
        T::NAME,
    );

    let mut stmt = connection.prepare(&sql)?;
    stmt.execute(value.as_params().as_slice())?;
    value.insert_foreign_references(connection)?;
    // for row in value.to_rows() {
    //     row.clone()
    //         .insert_into_connected_foreign_tables(true, connection)?;
    // }
    Ok(())
}

pub fn update_rows<'a, T: ToTable<'a> + HasPartial>(
    connection: &&'a rusqlite::Connection,
    filter: (),
    value: T::Partial,
) -> Result<(), rusqlite::Error>
where
    T::Partial: PartialRow,
{
    let columns: Vec<String> = value.used_column_names(None);
    if columns.is_empty() {
        return Ok(());
    }
    let columns_set = columns
        .into_iter()
        .enumerate()
        .map(|(i, c)| format!("{c} = ?{}", i + 1))
        .fold(String::new(), |mut acc: String, cur| {
            if acc.is_empty() {
                cur
            } else {
                acc.push_str(", ");
                acc.push_str(&cur);
                acc
            }
        });
    let mut sql = format!("UPDATE {} SET {columns_set}", T::NAME);
    sql.push(' ');
    // sql.push_str(&filter.to_sql());
    #[cfg(feature = "debug_sql")]
    dbg!(&sql);
    let mut statement = connection.prepare(&sql)?;
    let values: Vec<&dyn rusqlite::ToSql> = value.used_values();
    statement.execute(values.as_slice())?;
    Ok(())
}

fn load_where<'a, T: SqlTable<'a>>(
    connection: &rusqlite::Connection,
    filter: filter::GenericFilter,
) -> Result<Vec<T::RowType>, rusqlite::Error> {
    let columns = <T::RowType as ToTable>::columns()
        .into_iter()
        .map(|c| c.name)
        .fold(String::new(), |mut acc, cur| {
            if acc.is_empty() {
                cur.clone().into_owned()
            } else {
                acc.push_str(", ");
                acc.push_str(&cur);
                acc
            }
        });
    let mut sql = format!("SELECT {columns} from {}", T::RowType::NAME);
    filter.write_to(&mut sql, true);
    let mut statement = connection.prepare(&sql)?;
    Ok(statement
        .query_map((), |row| {
            Ok(
                T::RowType::try_from_row(row, &connection).unwrap_or_else(|err| {
                    dbg!(row);
                    panic!("Failed constructing value from row. sql was {sql}, err was {err}");
                }),
            )
        })?
        .collect::<Result<Vec<T::RowType>, _>>()?)
}

// pub fn query_table_filtered<'a, T: ToTable<'a>>(
//     connection: &&'a rusqlite::Connection,
//     filter: GenericFilter,
//     order: GenericOrder,
// ) -> Result<Vec<T>, rusqlite::Error> {
//     let columns = T::columns()
//         .into_iter()
//         .map(|c| c.name)
//         .fold(String::new(), |mut acc, cur| {
//             if acc.is_empty() {
//                 cur.clone().into_owned()
//             } else {
//                 acc.push_str(", ");
//                 acc.push_str(&cur);
//                 acc
//             }
//         });
//     // if !filter
//     //     .columns
//     //     .keys()
//     //     .into_iter()
//     //     .all(|k| T::columns().iter().any(|c| &c.name == k))
//     // {
//     //     todo!("Load missing tables?")
//     // }
//     let mut sql = format!("SELECT {columns} from {}", T::NAME);
//     sql.push(' ');
//     // sql.push_str(&filter.to_sql());
//     sql.push(' ');
//     sql.push_str(&order.to_sql());
//     #[cfg(feature = "debug_sql")]
//     dbg!(&sql);
//     let mut statement = connection.prepare(&sql)?;
//     Ok(statement
//         .query_map((), |row| {
//             Ok(T::try_from_row(row, &connection).unwrap_or_else(|| {
//                 #[cfg(feature = "debug_sql")]
//                 dbg!(row);
//                 panic!("Failed constructing value from row")
//             }))
//         })?
//         .collect::<Result<Vec<T>, _>>()?)
// }

// pub fn delete_table_filtered<'a, T: ToTable<'a>>(
//     connection: &&'a rusqlite::Connection,
//     filter: GenericFilter,
// ) -> Result<usize, rusqlite::Error> {
//     let mut sql = format!("DELETE FROM {}", T::NAME);
//     sql.push(' ');
//     // sql.push_str(&filter.to_sql());
//     #[cfg(feature = "debug_sql")]
//     dbg!(&sql);
//     let mut statement = connection.prepare(&sql)?;
//     Ok(statement.execute(())?)
// }
