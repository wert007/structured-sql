use std::{
    collections::HashMap,
    fmt::Debug,
    path::Path,
    sync::{Arc, Mutex},
};

pub use rusqlite;
use rusqlite::{Connection, types::Null};
pub use silo_derive::IntoSqlTable;

pub use konst;
use time::macros::format_description;
use time::{Date, Time};
use time::{OffsetDateTime, format_description::FormatItem};

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
    use std::{
        collections::HashMap,
        error::Error,
        sync::{Arc, Mutex},
    };

    use rusqlite::{Connection, OptionalExtension};

    use crate::{
        AsParams, Database, Filterable, FromRow, GenericFilter, GenericOrder,
        HasPartialRepresentation, HasValue, IntoGenericFilter, IntoSqlTable, MigrationHandler,
        PartialType, RowType, SqlColumn, SqlColumnFilter, SqlColumnType, SqlTable,
        StaticStringStorage, ToRows, handle_migration,
    };

    #[derive(Debug, PartialEq)]
    struct Coord {
        x: f64,
        y: f64,
    }

    impl ToRows<Coord> for Coord {
        fn to_rows(self) -> Vec<Coord> {
            vec![self]
        }
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
            _string_storage: &mut StaticStringStorage,
            _column_name: Option<&'static str>,
            _row: &rusqlite::Row,
            _connection: &rusqlite::Connection,
        ) -> Option<Self> {
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

    impl HasValue for PartialCoord {
        fn has_values(&self) -> bool {
            self.x.has_values() || self.y.has_values()
        }
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
        fn into_generic(
            self,
            _string_storage: &mut StaticStringStorage,
            _column_name: Option<&'static str>,
        ) -> GenericFilter {
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
        fn try_from_row(
            _string_storage: &mut StaticStringStorage,
            _column_name: Option<&'static str>,
            row: &rusqlite::Row,
            _connection: &rusqlite::Connection,
        ) -> Option<Self> {
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

    impl MigrationHandler for Coord {}

    impl HasPartialRepresentation for Coord {
        type Partial = PartialCoord;
    }

    impl Filterable for Coord {
        type Filtered = CoordFilter;

        fn must_be_equal(&self) -> Self::Filtered {
            CoordFilter::default()
                .x_should_be(self.x)
                .y_should_be(self.y)
        }

        fn must_contain(&self) -> Self::Filtered {
            CoordFilter::default()
                .x_should_be(self.x)
                .y_should_be(self.y)
        }
    }

    impl RowType for Coord {}

    impl<'a> IntoSqlTable<'a> for Coord {
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
        string_storage: Arc<Mutex<StaticStringStorage>>,
    }

    impl<'a> SqlTable<'a> for CoordTable<'a> {
        type RowType = Coord;
        type ValueType = Coord;

        fn insert(&self, row: impl ToRows<Self::RowType>) -> Result<(), rusqlite::Error> {
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

            let mut stmt = self.connection.prepare(&format!(
                "INSERT OR IGNORE INTO {} ({columns}) VALUES ({values})",
                Self::RowType::NAME
            ))?;
            for row in row.to_rows() {
                stmt.execute(row.as_params().as_slice())?;
            }
            Ok(())
        }

        fn filter(&self, filter: CoordFilter) -> Result<Vec<Coord>, rusqlite::Error> {
            let generic = filter.into_generic(&mut self.string_storage.lock().unwrap(), None);
            crate::query_table_filtered::<Self::RowType, Self::ValueType>(
                &self.connection,
                &mut self.string_storage.lock().unwrap(),
                generic,
                GenericOrder::default(),
            )
        }

        fn delete(&self, filter: CoordFilter) -> Result<usize, rusqlite::Error> {
            let generic = filter.into_generic(&mut self.string_storage.lock().unwrap(), None);
            crate::delete_table_filtered::<Self::RowType>(&self.connection, generic)
        }

        fn from_connection(
            connection: &'a Connection,
            string_storage: Arc<Mutex<StaticStringStorage>>,
        ) -> Self {
            Self {
                connection,
                string_storage,
            }
        }

        const INSERT_FAILURE_BEHAVIOR: crate::SqlFailureBehavior =
            crate::SqlFailureBehavior::Ignore;

        fn update(
            &self,
            filter: CoordFilter,
            updated: PartialCoord,
        ) -> Result<(), rusqlite::Error> {
            Err(rusqlite::Error::InvalidQuery)
        }

        fn migrate(&self, actual_columns: &[SqlColumn]) -> Result<(), rusqlite::Error> {
            handle_migration::<Self::RowType>(
                self.connection,
                &mut self.string_storage.lock().unwrap(),
                actual_columns,
            )
        }
    }
}

pub fn handle_migration<T: for<'a> IntoSqlTable<'a> + MigrationHandler + RowType>(
    connection: &rusqlite::Connection,
    string_storage: &mut StaticStringStorage,
    actual_columns: &[SqlColumn],
) -> Result<(), rusqlite::Error>
where
    <T as HasPartialRepresentation>::Partial: PartialType<T> + FromRow,
{
    let mut sql = "CREATE TABLE temporary".to_string();
    sql.push_str(" (");
    for (i, column) in T::COLUMNS.into_iter().enumerate() {
        if i > 0 {
            sql.push_str(",");
        }
        sql.push('"');
        sql.push_str(column.name);
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
            let partial = <T as HasPartialRepresentation>::Partial::try_from_row(
                string_storage,
                None,
                row,
                connection,
            )
            .unwrap_or_else(|| {
                dbg!(
                    row,
                    // <<T as HasPartialRepresentation>::Partial as Default>::default()
                );
                panic!("Partial should always match???")
            });
            Ok(<T as MigrationHandler>::migrate(
                string_storage,
                partial,
                row,
                connection,
            ))
        })?
        .filter_map(|x| x.ok().flatten())
        .collect();

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
    let values = (0..T::COLUMNS.len())
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
pub struct StaticStringStorage {
    values: Vec<&'static str>,
    capacities: Vec<usize>,
}

impl StaticStringStorage {
    pub fn store(&mut self, parts: &[&'static str]) -> &'static str {
        let value = parts.concat();
        self.store_heaped(value)
    }

    pub fn store_heaped(&mut self, value: String) -> &'static str {
        if let Some(v) = self.get(&value) {
            v
        } else {
            let capacity = value.capacity();
            let value = value.leak();
            self.values.push(value);
            self.capacities.push(capacity);
            value
        }
    }

    fn get(&self, value: &str) -> Option<&'static str> {
        self.values.iter().find(|v| v == &&value).copied()
    }

    pub fn new() -> Self {
        Self {
            values: Vec::new(),
            capacities: Vec::new(),
        }
    }
}

impl Drop for StaticStringStorage {
    fn drop(&mut self) {
        #[allow(unused_variables)]
        for (value, capacity) in self.values.iter().zip(&self.capacities) {
            #[allow(unused_unsafe)]
            unsafe {
                // DE-LEAK the leaked strings again! This actually invalidates
                // all references to the static references, we handed out, so it
                // is very much unsafe.
                // String::from_raw_parts(value.as_ptr() as *mut u8, value.len(), *capacity);
            }
        }
    }
}

pub struct Database {
    connection: rusqlite::Connection,
    static_string_storage: Arc<Mutex<StaticStringStorage>>,
}

impl Database {
    fn new_from_connection(connection: rusqlite::Connection) -> Self {
        Self {
            connection,
            static_string_storage: Arc::new(Mutex::new(StaticStringStorage::new())),
        }
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

    pub fn store(&mut self, parts: &[&'static str]) -> &'static str {
        self.static_string_storage.lock().unwrap().store(parts)
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
    pub fn load<'a, T: IntoSqlTable<'a>>(&'a self) -> rusqlite::Result<T::Table> {
        self.create::<T>()?;

        Ok(T::Table::from_connection(
            &self.connection,
            self.static_string_storage.clone(),
        ))
        // if self.table_exists(table_name)? {
        //     self.load_table::<T>(table_name)
        // } else {
        // }
    }

    fn create<'a, T: IntoSqlTable<'a>>(&'a self) -> Result<(), rusqlite::Error> {
        if self.connection.table_exists(None, T::NAME)? {
            return Ok(());
        }
        let mut sql = "CREATE TABLE IF NOT EXISTS ".to_string();
        sql.push_str(T::NAME);
        sql.push_str(" (");
        for (i, column) in T::COLUMNS.into_iter().enumerate() {
            if i > 0 {
                sql.push_str(",");
            }
            sql.push('"');
            sql.push_str(column.name);
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

    pub fn check<'a, T: IntoSqlTable<'a>>(&'a self) -> Result<(), rusqlite::Error> {
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
            let name = self
                .static_string_storage
                .lock()
                .unwrap()
                .store_heaped(name);
            columns.push(SqlColumn {
                name,
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
        if compare_columns(&columns, T::COLUMNS) {
            let table = self.load::<T>()?;
            table.migrate(&columns)?;
        }

        Ok(())
        // self.connection.pragma_query(schema_name, "table_info", f)
    }
}

#[derive(Debug)]
enum TableAlteration {
    InsertColumn(SqlColumn),
    ChangeType(usize, SqlColumnType),
    ChangeIsPrimary(usize, bool),
    ChangeIsUnique(usize, bool),
    DeleteColumn(&'static str),
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
        seen_columns.push(column.name);
        match find_by_name(actual, column.name) {
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
                necessary_alternations.push(TableAlteration::InsertColumn(*column));
            }
        }
    }
    for column in actual {
        if !seen_columns.contains(&column.name) {
            necessary_alternations.push(TableAlteration::DeleteColumn(column.name));
        }
    }
    return !necessary_alternations.is_empty();
}

pub trait AsRepeatedParams {
    const PARAM_COUNT: usize;
    fn as_params<'b>(&'b self) -> Vec<Vec<&'b dyn rusqlite::ToSql>>;
}

pub trait AsParams {
    const PARAM_COUNT: usize;
    fn as_params<'b>(&'b self) -> Vec<&'b dyn rusqlite::ToSql>;
}

impl<T: AsParams> AsRepeatedParams for T {
    const PARAM_COUNT: usize = T::PARAM_COUNT;

    fn as_params<'b>(&'b self) -> Vec<Vec<&'b dyn rusqlite::ToSql>> {
        vec![self.as_params()]
    }
}

impl<T: AsParams> AsRepeatedParams for Vec<T> {
    const PARAM_COUNT: usize = T::PARAM_COUNT;

    fn as_params<'b>(&'b self) -> Vec<Vec<&'b dyn rusqlite::ToSql>> {
        self.iter().map(|v| v.as_params()).collect()
    }
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
        impl PartialType<$t> for Option<$t> {
            fn transpose(self) -> Option<$t> {
                self
            }
        }

        impl RowType for $t {
            fn insert_into_connected_foreign_tables(
                self,
                is_top_level: bool,
                _: &rusqlite::Connection,
            ) -> rusqlite::Result<()> {
                Ok(())
            }
        }

        impl HasPartialRepresentation for $t {
            type Partial = Option<$t>;
        }
        impl IntoGenericFilter for SqlColumnFilter<$t> {
            fn into_generic(
                self,
                string_storage: &mut StaticStringStorage,
                column_name: Option<&'static str>,
            ) -> GenericFilter {
                GenericFilter {
                    columns: self
                        .into_sql_column_filter(
                            column_name.expect("has no sub columns, so it needs a column name"),
                            string_storage,
                        )
                        .into_iter()
                        .collect(),
                }
            }
        }

        impl Filterable for $t {
            type Filtered = SqlColumnFilter<$t>;

            fn must_be_equal(&self) -> Self::Filtered {
                SqlColumnFilter::MustBeEqual(self.clone())
            }

            fn must_contain(&self) -> Self::Filtered {
                SqlColumnFilter::Contains(self.clone())
            }
        }

        impl AsParams for $t {
            const PARAM_COUNT: usize = 1;
            fn as_params<'b>(&'b self) -> Vec<&'b dyn rusqlite::ToSql> {
                vec![self]
            }
        }

        impl FromRow for $t {
            fn try_from_row(
                _string_storage: &mut StaticStringStorage,
                column_name: Option<&'static str>,
                row: &rusqlite::Row,
                connection: &rusqlite::Connection,
            ) -> Option<Self> {
                row.get(column_name.expect("column name")).ok()
            }
        }

        impl IntoSqlColumnFilter for SqlColumnFilter<$t> {
            fn into_sql_column_filter(
                self,
                name: &'static str,
                _string_storage: &mut StaticStringStorage,
            ) -> Vec<(&'static str, SqlColumnFilter<SqlValue>)> {
                vec![(name, self.into_generic())]
            }
        }
    };
}

macro_rules! impl_as_params_and_nan_is_none {
    ($t:ty) => {
        impl RowType for $t {
            fn insert_into_connected_foreign_tables(
                self,
                is_top_level: bool,
                _: &rusqlite::Connection,
            ) -> rusqlite::Result<()> {
                Ok(())
            }
        }

        impl PartialType<$t> for Option<$t> {
            fn transpose(self) -> Option<$t> {
                self
            }
        }
        impl HasPartialRepresentation for $t {
            type Partial = Option<$t>;
        }
        impl IntoGenericFilter for SqlColumnFilter<$t> {
            fn into_generic(
                self,
                string_storage: &mut StaticStringStorage,
                column_name: Option<&'static str>,
            ) -> GenericFilter {
                GenericFilter {
                    columns: self
                        .into_sql_column_filter(
                            column_name.expect("has no sub columns, so it needs a column name"),
                            string_storage,
                        )
                        .into_iter()
                        .collect(),
                }
            }
        }

        impl Filterable for $t {
            type Filtered = SqlColumnFilter<$t>;

            fn must_be_equal(&self) -> Self::Filtered {
                SqlColumnFilter::MustBeEqual(self.clone())
            }

            fn must_contain(&self) -> Self::Filtered {
                SqlColumnFilter::Contains(self.clone())
            }
        }

        impl AsParams for $t {
            const PARAM_COUNT: usize = 1;
            fn as_params<'b>(&'b self) -> Vec<&'b dyn rusqlite::ToSql> {
                vec![self]
            }
        }

        impl FromRow for $t {
            fn try_from_row(
                _string_storage: &mut StaticStringStorage,
                column_name: Option<&'static str>,
                row: &rusqlite::Row,
                connection: &rusqlite::Connection,
            ) -> Option<Self> {
                Some(
                    row.get(column_name.expect("column name"))
                        .ok()
                        .unwrap_or(<$t>::NAN),
                )
            }
        }

        impl IntoSqlColumnFilter for SqlColumnFilter<$t> {
            fn into_sql_column_filter(
                self,
                name: &'static str,
                _string_storage: &mut StaticStringStorage,
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
                _string_storage: &mut StaticStringStorage,
            ) -> Vec<(&'static str, SqlColumnFilter<SqlValue>)> {
                vec![(name, self.into_generic())]
            }
        }
    };
}

impl_as_params!(bool);
impl_as_params!(Time);
impl_as_params!(Date);
impl_as_params!(OffsetDateTime);
impl_as_params!(i8);
impl_as_params!(i16);
impl_as_params!(i32);
impl_as_params!(i64);
impl_as_params!(isize);
impl_as_params!(u8);
impl_as_params!(u16);
impl_as_params!(u32);
impl_as_params!(usize);
impl_as_params!(u64);
impl_as_params_and_nan_is_none!(f32);
impl_as_params_and_nan_is_none!(f64);
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
related_sql_column_type!(SqlColumnType::Integer, isize);
related_sql_column_type!(SqlColumnType::Integer, u8);
related_sql_column_type!(SqlColumnType::Integer, u16);
related_sql_column_type!(SqlColumnType::Integer, u32);
related_sql_column_type!(SqlColumnType::Integer, u64);
related_sql_column_type!(SqlColumnType::Integer, usize);
related_sql_column_type!(SqlColumnType::Float, f32);
related_sql_column_type!(SqlColumnType::Float, f64);
related_sql_column_type!(SqlColumnType::Text, String);
related_sql_column_type!(SqlColumnType::Text, Time);
related_sql_column_type!(SqlColumnType::Text, Date);
related_sql_column_type!(SqlColumnType::Text, OffsetDateTime);

pub trait FromRow: Sized {
    fn try_from_row(
        string_storage: &mut StaticStringStorage,
        column_name: Option<&'static str>,
        row: &rusqlite::Row,
        connection: &rusqlite::Connection,
    ) -> Option<Self>;
}

pub trait PartialType<T> {
    fn transpose(self) -> Option<T>;
}

impl<T> PartialType<Option<T>> for Option<Option<T>>
// where
//     Option<T>: PartialType<T>,
{
    fn transpose(self) -> Option<Option<T>> {
        self
    }
}

impl<T> PartialType<Vec<T>> for Vec<T> {
    fn transpose(self) -> Option<Vec<T>> {
        if self.is_empty() {
            Some(Vec::new())
        } else {
            let mut result = Vec::with_capacity(self.len());
            for s in self {
                result.push(s);
            }
            Some(result)
        }
    }
}

pub trait HasValue {
    fn has_values(&self) -> bool;
}

pub trait HasPartialRepresentation<T = Self>: Sized {
    type Partial: HasValue + Default;
    // type Partial: PartialType<T>;
}

impl<T> HasValue for Option<T> {
    fn has_values(&self) -> bool {
        self.is_some()
    }
}

impl<T: HasValue + Default> HasPartialRepresentation for T {
    type Partial = T;
}

// impl<T: HasPartialRepresentation> HasPartialRepresentation for Option<T>
// // where
// //     Option<<T as HasPartialRepresentation>::Partial>: From<Option<T>>,
// {
//     type Partial = Option<T::Partial>;
// }

impl<T> HasValue for Vec<T> {
    fn has_values(&self) -> bool {
        !self.is_empty()
    }
}

pub trait MigrationHandler: Sized + HasPartialRepresentation
where
    <Self as HasPartialRepresentation>::Partial: PartialType<Self>,
{
    fn migrate(
        string_storage: &mut StaticStringStorage,
        partial: Self::Partial,
        row: &rusqlite::Row,
        connection: &rusqlite::Connection,
    ) -> Option<Self> {
        partial.transpose()
    }
}

impl<T: FromRow> FromRow for Option<T> {
    fn try_from_row(
        string_storage: &mut StaticStringStorage,
        column_name: Option<&'static str>,
        row: &rusqlite::Row,
        connection: &rusqlite::Connection,
    ) -> Option<Self> {
        match T::try_from_row(string_storage, column_name, row, connection) {
            Some(it) => Some(Some(it)),
            None => Some(None),
        }
    }
}

pub trait FromGroupedRows: Sized {
    type RowType: FromRow;
    fn try_from_rows(
        string_storage: &mut StaticStringStorage,
        column_name: Option<&'static str>,
        rows: Vec<Self::RowType>,
    ) -> Option<Self>;
}

impl<T: FromRow> FromGroupedRows for Vec<T> {
    fn try_from_rows(
        string_storage: &mut StaticStringStorage,
        column_name: Option<&'static str>,
        rows: Vec<Self::RowType>,
    ) -> Option<Self> {
        Some(rows)
    }

    type RowType = T;
}

pub trait RowType: FromRow + AsParams + HasPartialRepresentation + Filterable {
    fn insert_into_connected_foreign_tables(
        self,
        is_top_level: bool,
        connection: &rusqlite::Connection,
    ) -> rusqlite::Result<()> {
        Ok(())
    }
}

impl<T: RowType> RowType for Option<T> {
    fn insert_into_connected_foreign_tables(
        self,
        is_top_level: bool,
        connection: &rusqlite::Connection,
    ) -> rusqlite::Result<()> {
        if let Some(it) = self {
            it.insert_into_connected_foreign_tables(is_top_level, connection)?;
        }
        Ok(())
    }
}

impl<T: RowType> FromRowType<Self> for T {
    fn from_row_type(value: Vec<Self>) -> Vec<Self> {
        value
    }
}

pub trait FromRowType<T: RowType>:
    MustBeEqual<T::Filtered> + Sized + HasPartialRepresentation
{
    fn from_row_type(value: Vec<T>) -> Vec<Self>;
}

pub trait MustBeEqual<T: IntoGenericFilter> {
    fn must_be_equal(&self) -> T;
}

pub trait IntoSqlTable<'a> {
    const COLUMNS: &'static [SqlColumn];
    const NAME: &'static str;
    type Table: SqlTable<'a>;
}

impl<'a, T: IntoSqlTable<'a>> IntoSqlTable<'a> for Option<T>
// where
//     Option<<T as HasPartialRepresentation>::Partial>: From<Option<T>>,
{
    const COLUMNS: &'static [SqlColumn] = T::COLUMNS;

    const NAME: &'static str = T::NAME;

    type Table = T::Table;
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

impl ToString for SqlFailureBehavior {
    fn to_string(&self) -> String {
        match self {
            SqlFailureBehavior::Abort => "ABORT".to_string(),
            SqlFailureBehavior::Fail => "FAIL".to_string(),
            SqlFailureBehavior::Ignore => "IGNORE".to_string(),
            SqlFailureBehavior::Replace => "REPLACE".to_string(),
            SqlFailureBehavior::Rollback => "ROLLBACK".to_string(),
        }
    }
}

pub trait ToRows<T> {
    fn to_rows(self) -> Vec<T>;
}

impl<T> ToRows<T> for Vec<T> {
    fn to_rows(self) -> Vec<T> {
        self
    }
}

pub trait SqlTable<'a> {
    type RowType: RowType;
    type ValueType: FromRowType<Self::RowType>;
    const INSERT_FAILURE_BEHAVIOR: SqlFailureBehavior;
    fn from_connection(
        connection: &'a Connection,
        string_storage: Arc<Mutex<StaticStringStorage>>,
    ) -> Self;
    fn filter(
        &self,
        filter: <Self::RowType as Filterable>::Filtered,
    ) -> Result<Vec<Self::ValueType>, rusqlite::Error>;
    fn delete(
        &self,
        filter: <Self::RowType as Filterable>::Filtered,
    ) -> Result<usize, rusqlite::Error>;

    fn insert(&self, row: impl ToRows<Self::RowType>) -> Result<(), rusqlite::Error>;
    fn update(
        &self,
        filter: <Self::RowType as Filterable>::Filtered,
        updated: <Self::ValueType as HasPartialRepresentation>::Partial,
    ) -> Result<(), rusqlite::Error>;
    fn count(
        &self,
        filter: <Self::RowType as Filterable>::Filtered,
    ) -> Result<usize, rusqlite::Error> {
        Ok(self.filter(filter)?.len())
    }
    fn migrate(&self, actual_columns: &[SqlColumn]) -> Result<(), rusqlite::Error>;
    fn drain(
        &self,
        mut callback: impl FnMut(&Self::ValueType) -> bool,
    ) -> Result<Vec<Self::ValueType>, rusqlite::Error> {
        Ok(self
            .filter(Default::default())?
            .into_iter()
            .filter_map(|r| {
                if (callback)(&r) {
                    self.delete(r.must_be_equal()).ok()?;
                    Some(r)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>())
    }
}

pub trait FromRowWithPrimary: FromRow {
    fn primary(&self) -> usize;
}

#[derive(Default, Clone, Debug)]
pub enum SqlColumnFilter<T: Clone + std::fmt::Debug> {
    #[default]
    Ignored,
    MustBeEqual(T),
    Contains(T),
}

impl<T: Into<SqlValue> + Clone + std::fmt::Debug> SqlColumnFilter<T> {
    pub fn into_generic(self) -> SqlColumnFilter<SqlValue> {
        match self {
            SqlColumnFilter::Ignored => SqlColumnFilter::Ignored,
            SqlColumnFilter::MustBeEqual(it) => SqlColumnFilter::MustBeEqual(it.into()),
            SqlColumnFilter::Contains(it) => SqlColumnFilter::Contains(it.into()),
        }
    }
}

impl SqlColumnFilter<SqlValue> {
    pub fn to_sql(&self) -> String {
        match self {
            SqlColumnFilter::Ignored => unreachable!(),
            SqlColumnFilter::MustBeEqual(v) => format!(" = {}", v.to_sql()),
            SqlColumnFilter::Contains(v) => {
                let string_representation = v.to_sql();
                if string_representation.starts_with('\'') && string_representation.ends_with('\'')
                {
                    format!(
                        " LIKE '%{}%'",
                        &string_representation[1..string_representation.len() - 1]
                    )
                } else {
                    // Fallback to must be equal
                    format!(" = {}", v.to_sql())
                }
            }
        }
    }
}

pub trait IntoSqlColumnFilter {
    fn into_sql_column_filter(
        self,
        name: &'static str,
        string_storage: &mut StaticStringStorage,
    ) -> Vec<(&'static str, SqlColumnFilter<SqlValue>)>;
}

impl<T: IntoSqlColumnFilter + Clone + Debug> IntoSqlColumnFilter for SqlColumnFilter<T> {
    fn into_sql_column_filter(
        self,
        name: &'static str,
        string_storage: &mut StaticStringStorage,
    ) -> Vec<(&'static str, SqlColumnFilter<SqlValue>)> {
        match self {
            SqlColumnFilter::Ignored => vec![],
            SqlColumnFilter::MustBeEqual(t) => t.into_sql_column_filter(name, string_storage),
            // TODO: This should probably more behave like (does any column of this type have tis value)
            SqlColumnFilter::Contains(t) => t.into_sql_column_filter(name, string_storage),
        }
    }
}

pub trait Filterable {
    type Filtered: IntoGenericFilter + Default;

    fn must_be_equal(&self) -> Self::Filtered;
    fn must_contain(&self) -> Self::Filtered;
}

impl<F: Filterable> MustBeEqual<F::Filtered> for F {
    fn must_be_equal(&self) -> F::Filtered {
        self.must_be_equal()
    }
}

impl<T: IntoGenericFilter + Default + Clone> Filterable for T {
    type Filtered = T;

    fn must_be_equal(&self) -> Self::Filtered {
        self.clone()
    }
    fn must_contain(&self) -> Self::Filtered {
        self.clone()
    }
}

impl<T: Filterable> Filterable for Option<T> {
    type Filtered = T::Filtered;

    fn must_be_equal(&self) -> Self::Filtered {
        match self {
            Some(it) => it.must_be_equal(),
            None => Default::default(),
        }
    }

    fn must_contain(&self) -> Self::Filtered {
        match self {
            Some(it) => it.must_contain(),
            None => Default::default(),
        }
    }
}

impl<T: Filterable> Filterable for Vec<T> {
    type Filtered = T::Filtered;

    fn must_be_equal(&self) -> Self::Filtered {
        // self.unwrap().must_be_equal()
        todo!()
    }

    fn must_contain(&self) -> Self::Filtered {
        todo!()
    }
}

pub trait IntoGenericFilter {
    fn into_generic(
        self,
        string_storage: &mut StaticStringStorage,
        column_name: Option<&'static str>,
    ) -> GenericFilter;
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

pub struct GenericFilter {
    pub columns: HashMap<&'static str, SqlColumnFilter<SqlValue>>,
}

impl GenericFilter {
    pub fn insert_into_columns(
        name: &'static str,
        columns: &mut HashMap<&'static str, SqlColumnFilter<SqlValue>>,
        value: impl IntoSqlColumnFilter,
        string_storage: &mut StaticStringStorage,
    ) {
        let values = value.into_sql_column_filter(name, string_storage);
        for (name, value) in values {
            columns.insert(name, value.clone());
        }
    }

    fn get_params(&self) -> () {
        ()
    }

    fn to_sql(&self) -> String {
        use std::fmt::Write;
        if !self
            .columns
            .iter()
            .any(|c| !matches!(c.1, SqlColumnFilter::Ignored))
        {
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
        if self.has_values() {
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

impl<T: AsParams> PartialRow for Vec<T> {
    fn used_column_names(&self, column_name: Option<String>) -> Vec<String> {
        if self.has_values() {
            vec![column_name.expect("Needs column name!")]
        } else {
            Vec::new()
        }
    }

    fn used_values(&self) -> Vec<&dyn rusqlite::ToSql> {
        if self.len() > 1 {
            eprintln!("Ahh, this is lossy!");
        }
        if let Some(value) = self.into_iter().next() {
            value.as_params()
        } else {
            Vec::new()
        }
    }
}

pub fn update_rows<'a, T: IntoSqlTable<'a> + RowType>(
    connection: &&'a rusqlite::Connection,
    filter: GenericFilter,
    value: impl ToRows<T::Partial>,
) -> Result<(), rusqlite::Error>
where
    T::Partial: PartialRow,
{
    let values = value.to_rows();
    if values.is_empty() {
        return Ok(());
    }
    let columns: Vec<String> = values[0].used_column_names(None);
    if columns.is_empty() {
        return Ok(());
    }
    let columns_set = columns
        .into_iter()
        .enumerate()
        .map(|(i, c)| format!("{c} = ?{}", i + 1))
        .fold(String::new(), |mut acc, cur| {
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
    sql.push_str(&filter.to_sql());
    #[cfg(feature = "debug_sql")]
    dbg!(&sql);
    let mut statement = connection.prepare(&sql)?;
    for value in values {
        let values: Vec<&dyn rusqlite::ToSql> = value.used_values();
        statement.execute(values.as_slice())?;
    }
    Ok(())
}

pub fn query_table_filtered<'a, T: IntoSqlTable<'a> + RowType, U: FromRowType<T>>(
    connection: &&'a rusqlite::Connection,
    string_storage: &mut StaticStringStorage,
    filter: GenericFilter,
    order: GenericOrder,
) -> Result<Vec<U>, rusqlite::Error> {
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
    if !filter
        .columns
        .keys()
        .into_iter()
        .all(|k| T::COLUMNS.iter().any(|c| &c.name == k))
    {
        todo!("Load missing tables?")
    }
    let mut sql = format!("SELECT {columns} from {}", T::NAME);
    sql.push(' ');
    sql.push_str(&filter.to_sql());
    sql.push(' ');
    sql.push_str(&order.to_sql());
    #[cfg(feature = "debug_sql")]
    dbg!(&sql);
    let mut statement = connection.prepare(&sql)?;
    Ok(FromRowType::from_row_type(
        statement
            .query_map(filter.get_params(), |row| {
                Ok(
                    T::try_from_row(string_storage, None, row, &connection).unwrap_or_else(|| {
                        #[cfg(feature = "debug_sql")]
                        dbg!(row);
                        panic!("Failed constructing value from row")
                    }),
                )
            })?
            .collect::<Result<Vec<T>, _>>()?,
    ))
}

pub fn delete_table_filtered<'a, T: IntoSqlTable<'a>>(
    connection: &&'a rusqlite::Connection,
    filter: GenericFilter,
) -> Result<usize, rusqlite::Error> {
    let mut sql = format!("DELETE FROM {}", T::NAME);
    sql.push(' ');
    sql.push_str(&filter.to_sql());
    #[cfg(feature = "debug_sql")]
    dbg!(&sql);
    let mut statement = connection.prepare(&sql)?;
    Ok(statement.execute(filter.get_params())?)
}
