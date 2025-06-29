use std::{
    collections::HashMap,
    ffi::{CStr, CString},
    fmt::Debug,
    path::Path,
};

pub use rusqlite;
use rusqlite::{Connection, Name, OpenFlags, types::Null};
pub use structured_sql_derive::IntoSqlTable;

pub use konst;

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
        let coords = coords.filter(CoordFilter::default().y_should_be(13324.3124))?;
        assert_eq!(coords.as_slice(), &[]);
        Ok(())
    }
    use std::{collections::HashMap, error::Error};

    use facet::Facet;
    use rusqlite::{Connection, OptionalExtension};

    use crate::{
        AsParams, Database, FromRow, GenericFilter, IntoGenericFilter, IntoSqlTable, SqlColumn,
        SqlColumnFilter, SqlColumnType, SqlTable,
    };

    #[derive(Debug, PartialEq, Facet)]
    struct Coord {
        x: f64,
        y: f64,
    }

    #[derive(Default)]
    struct CoordFilter {
        x: Option<SqlColumnFilter<f64>>,
        y: Option<SqlColumnFilter<f64>>,
    }

    impl CoordFilter {
        pub fn x_should_be(mut self, x: f64) -> Self {
            self.x = Some(SqlColumnFilter::MustBeEqual(x));
            self
        }

        pub fn y_should_be(mut self, y: f64) -> Self {
            self.y = Some(SqlColumnFilter::MustBeEqual(y));
            self
        }
    }

    impl IntoGenericFilter for CoordFilter {
        fn into_generic(self, column_name: Option<&'static str>) -> GenericFilter {
            let mut columns = HashMap::new();
            if let Some(x) = self.x {
                columns.insert("x", x.into_generic());
            }
            if let Some(y) = self.y {
                columns.insert("y", y.into_generic());
            }
            GenericFilter { columns }
        }
    }

    impl FromRow for Coord {
        fn from_row(row_name: Option<&'static str>, row: &rusqlite::Row) -> Self {
            let x: f64 = row
                .get("x")
                .optional()
                .unwrap()
                .expect("Implement missing columns");
            let y: f64 = row
                .get("y")
                .optional()
                .unwrap()
                .expect("Implement missing columns");
            Self { x, y }
        }

        fn try_from_row(column_name: Option<&'static str>, row: &rusqlite::Row) -> Option<Self> {
            let x: f64 = row.get("x").optional().unwrap()?;
            let y: f64 = row.get("y").optional().unwrap()?;
            Some(Self { x, y })
        }
    }

    impl AsParams for Coord {
        const PARAM_COUNT: usize = 2;

        fn as_params<'b>(&'b self) -> Vec<&'b dyn rusqlite::ToSql> {
            vec![&self.x, &self.y]
        }
    }

    impl<'a> IntoSqlTable<'a> for Coord {
        type Filter = CoordFilter;
        type Table = CoordTable<'a>;
        const COLUMNS: &'static [crate::SqlColumn] = &[
            SqlColumn {
                name: "x",
                r#type: SqlColumnType::Float,
                is_primary: false,
                is_unique: false,
            },
            SqlColumn {
                name: "y",
                r#type: SqlColumnType::Float,
                is_primary: false,
                is_unique: false,
            },
        ];

        const NAME: &'static str = "CoordTable";
    }

    struct CoordTable<'a> {
        connection: &'a Connection,
    }

    impl<'a> SqlTable<'a> for CoordTable<'a> {
        type RowType = Coord;

        fn insert(&self, row: Self::RowType) -> Result<(), rusqlite::Error> {
            let columns = Self::RowType::COLUMNS.into_iter().map(|c| c.name).fold(
                String::new(),
                |mut acc, cur| {
                    if acc.is_empty() {
                        cur.into()
                    } else {
                        acc.push_str(", ");
                        acc.push_str(cur);
                        acc
                    }
                },
            );
            let values = (0..Self::RowType::COLUMNS.len()).map(|v| v + 1).fold(
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
            self.connection.execute(
                &format!(
                    "INSERT INTO {} ({columns}) VALUES ({values})",
                    Self::RowType::NAME
                ),
                row.as_params().as_slice(),
            )?;
            Ok(())
        }

        fn filter(&self, filter: CoordFilter) -> Result<Vec<Coord>, rusqlite::Error> {
            let generic = filter.into_generic(None);
            crate::query_table_filtered::<Self::RowType>(&self.connection, generic)
        }

        fn from_connection(connection: &'a Connection) -> Self {
            Self { connection }
        }
    }
}

pub struct Database {
    connection: rusqlite::Connection,
}

impl Database {
    pub fn create_in_memory() -> Result<Self, rusqlite::Error> {
        let connection = rusqlite::Connection::open_in_memory()?;
        Ok(Self { connection })
    }
    pub fn open(path: impl AsRef<Path>) -> Result<Self, rusqlite::Error> {
        let connection = rusqlite::Connection::open(path)?;
        Ok(Self { connection })
    }

    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), rusqlite::Error> {
        self.connection.backup("main", path, None)?;
        Ok(())
    }
    pub fn load<'a, T: IntoSqlTable<'a>>(&'a self) -> rusqlite::Result<T::Table> {
        self.create::<T>()?;

        Ok(T::Table::from_connection(&self.connection))
        // if self.table_exists(table_name)? {
        //     self.load_table::<T>(table_name)
        // } else {
        // }
    }

    // fn table_exists(&self, table_name: &str) -> rusqlite::Result<bool> {
    //     let mut exists = self
    //         .connection
    //         .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name=?1;")?;
    //     Ok(exists
    //         .query_row((table_name,), |r| Ok(()))
    //         .optional()?
    //         .is_some())
    // }

    fn create<'a, T: IntoSqlTable<'a>>(&'a self) -> Result<(), rusqlite::Error> {
        let mut sql = "CREATE TABLE IF NOT EXISTS ".to_string();
        sql.push_str(T::NAME);
        sql.push_str(" (");
        for (i, column) in T::COLUMNS.into_iter().enumerate() {
            if i > 0 {
                sql.push_str(",");
            }
            sql.push_str(column.name);
            sql.push_str(" ");
            sql.push_str(column.r#type.as_sql());
        }
        sql.push_str(");");
        self.connection.execute(&sql, ())?;
        Ok(())
    }

    // fn load_table<T: IntoSqlTable>(
    //     &self,
    //     table_name: &str,
    // ) -> Result<<T as IntoSqlTable>::Table, rusqlite::Error> {
    //     todo!()
    // }
}

pub trait AsParams {
    const PARAM_COUNT: usize;
    fn as_params<'b>(&'b self) -> Vec<&'b dyn rusqlite::ToSql>;
}

impl<T: AsParams> AsParams for Option<T> {
    fn as_params<'b>(&'b self) -> Vec<&'b dyn rusqlite::ToSql> {
        match self {
            Some(it) => it.as_params(),
            None => vec![&Null; T::PARAM_COUNT],
        }
    }

    const PARAM_COUNT: usize = T::PARAM_COUNT;
}

macro_rules! impl_as_params {
    ($t:ty) => {
        impl IntoGenericFilter for SqlColumnFilter<$t> {
            fn into_generic(self, column_name: Option<&'static str>) -> GenericFilter {
                GenericFilter {
                    columns: self
                        .into_sql_column_filter(
                            column_name.expect("has no sub columns, so it needs a column name"),
                        )
                        .into_iter()
                        .collect(),
                }
            }
        }

        impl Filterable for $t {
            type Filtered = SqlColumnFilter<$t>;

            fn must_be_equal(self) -> Self::Filtered {
                SqlColumnFilter::MustBeEqual(self)
            }
        }

        impl AsParams for $t {
            const PARAM_COUNT: usize = 1;
            fn as_params<'b>(&'b self) -> Vec<&'b dyn rusqlite::ToSql> {
                vec![self]
            }
        }

        impl FromRow for $t {
            fn from_row(column_name: Option<&'static str>, row: &rusqlite::Row) -> Self {
                Self::try_from_row(column_name, row).expect("Value")
            }

            fn try_from_row(
                column_name: Option<&'static str>,
                row: &rusqlite::Row,
            ) -> Option<Self> {
                use rusqlite::OptionalExtension;

                match row.get(column_name.expect("column name")).optional() {
                    Ok(it) => it,
                    Err(rusqlite::Error::InvalidColumnType(_, _, rusqlite::types::Type::Null)) => {
                        None
                    }
                    Err(err) => {
                        unreachable!("Expected no errors here: {err}");
                    }
                }
            }
        }

        impl IntoSqlColumnFilter for SqlColumnFilter<$t> {
            fn into_sql_column_filter(
                self,
                name: &'static str,
            ) -> Vec<(&'static str, SqlColumnFilter<SqlValue>)> {
                vec![(name, self.into_generic())]
            }
        }
    };
}
macro_rules! impl_as_params_and_column_filter {
    ($t:ty) => {
        impl AsParams for $t {
            const PARAM_COUNT: usize = 1;
            fn as_params<'b>(&'b self) -> Vec<&'b dyn rusqlite::ToSql> {
                vec![self]
            }
        }

        impl IntoSqlColumnFilter for SqlColumnFilter<$t> {
            fn into_sql_column_filter(
                self,
                name: &'static str,
            ) -> Vec<(&'static str, SqlColumnFilter<SqlValue>)> {
                vec![(name, self.into_generic())]
            }
        }
    };
}

impl_as_params!(bool);
impl_as_params!(i8);
impl_as_params!(i16);
impl_as_params!(i32);
impl_as_params!(i64);
impl_as_params!(u8);
impl_as_params!(u16);
impl_as_params!(u32);
impl_as_params!(u64);
impl_as_params!(f32);
impl_as_params!(f64);
impl_as_params!(String);
impl_as_params_and_column_filter!(&str);

pub trait RelatedSqlColumnType {
    const SQL_COLUMN_TYPE: SqlColumnType;
}

impl<T: RelatedSqlColumnType> RelatedSqlColumnType for Option<T> {
    const SQL_COLUMN_TYPE: SqlColumnType = SqlColumnType::to_optional(T::SQL_COLUMN_TYPE);
}

macro_rules! related_sql_column_type {
    ($v:path, $t:ty) => {
        impl RelatedSqlColumnType for $t {
            const SQL_COLUMN_TYPE: SqlColumnType = $v;
        }
    };
}

related_sql_column_type!(SqlColumnType::Integer, bool);
related_sql_column_type!(SqlColumnType::Integer, i8);
related_sql_column_type!(SqlColumnType::Integer, i16);
related_sql_column_type!(SqlColumnType::Integer, i32);
related_sql_column_type!(SqlColumnType::Integer, i64);
related_sql_column_type!(SqlColumnType::Integer, u8);
related_sql_column_type!(SqlColumnType::Integer, u16);
related_sql_column_type!(SqlColumnType::Integer, u32);
related_sql_column_type!(SqlColumnType::Integer, u64);
related_sql_column_type!(SqlColumnType::Float, f32);
related_sql_column_type!(SqlColumnType::Float, f64);
related_sql_column_type!(SqlColumnType::Text, String);

pub trait FromRow: Sized {
    fn from_row(column_name: Option<&'static str>, row: &rusqlite::Row) -> Self;
    fn try_from_row(column_name: Option<&'static str>, row: &rusqlite::Row) -> Option<Self>;
}

impl<T: FromRow> FromRow for Option<T> {
    fn from_row(column_name: Option<&'static str>, row: &rusqlite::Row) -> Self {
        T::try_from_row(column_name, row)
    }

    fn try_from_row(column_name: Option<&'static str>, row: &rusqlite::Row) -> Option<Self> {
        match T::try_from_row(column_name, row) {
            Some(it) => Some(Some(it)),
            None => None,
        }
    }
}

pub trait IntoSqlTable<'a>: FromRow + AsParams {
    const COLUMNS: &'static [SqlColumn];
    const NAME: &'static str;
    type Table: SqlTable<'a>;
    type Filter: IntoGenericFilter;

    // fn table_as_params<'b>(&'b self) -> Vec<&'b dyn rusqlite::ToSql>;
}

impl<'a, T: IntoSqlTable<'a>> IntoSqlTable<'a> for Option<T> {
    const COLUMNS: &'static [SqlColumn] = T::COLUMNS;

    const NAME: &'static str = T::NAME;

    type Table = T::Table;

    type Filter = T::Filter;

    // fn table_as_params<'b>(&'b self) -> Vec<&'b dyn rusqlite::ToSql> {
    //     unreachable!()
    // }
}

pub trait SqlTable<'a> {
    type RowType: IntoSqlTable<'a>;
    fn from_connection(connection: &'a Connection) -> Self;
    fn filter(
        &self,
        filter: <Self::RowType as IntoSqlTable<'a>>::Filter,
    ) -> Result<Vec<Self::RowType>, rusqlite::Error>;
    fn insert(&self, row: Self::RowType) -> Result<(), rusqlite::Error>;
}

#[derive(Default, Clone, Debug)]
pub enum SqlColumnFilter<T: Clone + std::fmt::Debug> {
    #[default]
    Ignored,
    MustBeEqual(T),
}

impl<T: Into<SqlValue> + Clone + std::fmt::Debug> SqlColumnFilter<T> {
    pub fn into_generic(self) -> SqlColumnFilter<SqlValue> {
        match self {
            SqlColumnFilter::Ignored => SqlColumnFilter::Ignored,
            SqlColumnFilter::MustBeEqual(it) => SqlColumnFilter::MustBeEqual(it.into()),
        }
    }
}

impl SqlColumnFilter<SqlValue> {
    pub fn to_sql(&self) -> String {
        match self {
            SqlColumnFilter::Ignored => unreachable!(),
            SqlColumnFilter::MustBeEqual(v) => format!(" = {}", v.to_sql()),
        }
    }
}

pub trait IntoSqlColumnFilter {
    fn into_sql_column_filter(
        self,
        name: &'static str,
    ) -> Vec<(&'static str, SqlColumnFilter<SqlValue>)>;
}

impl<T: IntoSqlColumnFilter + Clone + Debug> IntoSqlColumnFilter for SqlColumnFilter<T> {
    fn into_sql_column_filter(
        self,
        name: &'static str,
    ) -> Vec<(&'static str, SqlColumnFilter<SqlValue>)> {
        match self {
            SqlColumnFilter::Ignored => vec![],
            SqlColumnFilter::MustBeEqual(t) => t.into_sql_column_filter(name),
        }
    }
}

pub trait Filterable {
    type Filtered: IntoGenericFilter;

    fn must_be_equal(self) -> Self::Filtered;
}

impl<T: Filterable> Filterable for Option<T> {
    type Filtered = T::Filtered;

    fn must_be_equal(self) -> Self::Filtered {
        self.unwrap().must_be_equal()
    }
}

pub trait IntoGenericFilter {
    fn into_generic(self, column_name: Option<&'static str>) -> GenericFilter;
}

pub struct GenericFilter {
    pub columns: HashMap<&'static str, SqlColumnFilter<SqlValue>>,
}

impl GenericFilter {
    pub fn insert_into_columns(
        name: &'static str,
        columns: &mut HashMap<&'static str, SqlColumnFilter<SqlValue>>,
        value: impl IntoSqlColumnFilter,
    ) {
        let values = value.into_sql_column_filter(name);
        for (name, value) in values {
            columns.insert(name, value.clone());
        }
    }

    fn get_params(&self) -> () {
        ()
    }

    fn to_sql(&self) -> String {
        use std::fmt::Write;
        if self.columns.is_empty() {
            return String::new();
        }
        let mut result: String = "WHERE".into();
        let mut emitted = false;
        for (name, filter) in &self.columns {
            if matches!(filter, SqlColumnFilter::Ignored) {
                continue;
            }
            if emitted {
                write!(result, " AND").expect("Infallibe");
            }
            write!(result, " {name} {}", filter.to_sql()).expect("Infallible");
            emitted = true;
        }
        result
    }
}

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
into_sql_value_integer!(u8);
into_sql_value_integer!(u16);
into_sql_value_integer!(u32);
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

impl SqlValue {
    fn to_sql(&self) -> String {
        match self {
            SqlValue::Float(it) => it.to_string(),
            SqlValue::Integer(it) => it.to_string(),
            SqlValue::Null => "NULL".to_string(),
            SqlValue::Text(it) => format!("{it:?}"),
            SqlValue::Blob(items) => todo!(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SqlColumn {
    pub name: &'static str,
    pub r#type: SqlColumnType,
    pub is_primary: bool,
    pub is_unique: bool,
}

pub trait HasSqlColumnType {
    const TYPE: SqlColumnType;
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
    fn as_sql(&self) -> &'static str {
        match self {
            SqlColumnType::Float => "REAL NOT NULL",
            SqlColumnType::Integer => "INTEGER NOT NULL",
            SqlColumnType::Null => "NULL",
            SqlColumnType::Text => "TEXT NOT NULL",
            SqlColumnType::Blob => "BLOB NOT NULL",
            SqlColumnType::OptionalFloat => "REAL",
            SqlColumnType::OptionalInteger => "INTEGER",
            SqlColumnType::OptionalText => "TEXT",
            SqlColumnType::OptionalBlob => "BLOB",
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

pub trait Convert<T> {
    fn convert() -> SqlColumnType;
}

impl Convert<f64> for SqlColumnType {
    fn convert() -> SqlColumnType {
        SqlColumnType::Float
    }
}

pub fn query_table_filtered<'a, T: IntoSqlTable<'a>>(
    connection: &&'a rusqlite::Connection,
    filter: GenericFilter,
) -> Result<Vec<T>, rusqlite::Error> {
    let columns = T::COLUMNS
        .into_iter()
        .map(|c| c.name)
        .fold(String::new(), |mut acc, cur| {
            if acc.is_empty() {
                cur.into()
            } else {
                acc.push_str(", ");
                acc.push_str(cur);
                acc
            }
        });
    let mut sql = format!("SELECT {columns} from {}", T::NAME);
    sql.push(' ');
    sql.push_str(&filter.to_sql());
    let mut statement = connection.prepare(&sql)?;
    Ok(statement
        .query_map(filter.get_params(), |row| Ok(T::from_row(None, row)))?
        .collect::<Result<_, _>>()?)
}
