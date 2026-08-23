mod table_info;

use crate::{FromRow, SqlColumn, SqlColumnType, SqlTable, ToTable};
pub use table_info::TableInfo;

#[derive(Debug, thiserror::Error)]
pub enum MigrationError {
    #[error("Unexpected error occured: {0}")]
    Rusqlite(#[from] rusqlite::Error),
}

impl From<crate::Error> for MigrationError {
    fn from(value: crate::Error) -> Self {
        todo!()
    }
}

pub fn check_table_schema<'a, T: ToTable<'a>>(table: &T::Table) -> Result<(), MigrationError> {
    let existing_schema = Schema::from_connection(table.connection(), &T::table_name())?;
    let desired_schema = Schema::from_definition::<T>();
    if !existing_schema.is_diff(&desired_schema) {
        return Ok(());
    }
    dbg!(existing_schema, desired_schema);
    todo!()
}

#[derive(Debug)]
struct Schema {
    columns: Vec<SqlColumn>,
}

impl Schema {
    fn is_diff(&self, other: &Self) -> bool {
        if self.columns.len() != other.columns.len() {
            return false;
        }
        let mut self_columns = self.columns.clone();
        self_columns.sort_by_cached_key(|c| c.name.clone().into_owned());
        let mut other_columns = other.columns.clone();
        other_columns.sort_by_key(|c| c.name().to_string());
        self_columns
            .into_iter()
            .zip(other_columns)
            .any(|(s, o)| s != o)
    }
    fn from_connection(
        connection: &rusqlite::Connection,
        table_name: &str,
    ) -> Result<Self, MigrationError> {
        let mut s = connection.prepare(&format!("PRAGMA table_info(\"{table_name}\");"))?;
        let table_columns: Vec<table_info::TableInfo> = s
            .query(())?
            .mapped(|r| Ok(table_info::TableInfo::try_from_row(r, connection)))
            .map(|e| match e {
                Ok(Ok(it)) => Ok::<TableInfo, crate::Error>(it),
                Ok(Err(e)) => Err(e.into()),
                Err(e) => Err(e.into()),
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mut s = connection.prepare(&format!("PRAGMA index_list(\"{table_name}\");"))?;
        let table_indices = s
            .query(())?
            .mapped(|r| {
                let name: String = r.get(1usize)?;
                let is_unique: u32 = r.get(2)?;
                Ok((name, is_unique != 0))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let uniqueness = table_indices
            .into_iter()
            .filter(|i| i.1)
            .map(|i| {
                let mut s = connection.prepare(&format!("PRAGMA index_xinfo(\"{}\");", i.0))?;
                s.query_map((), |r| {
                    let Ok(name) = r.get(2) else {
                        return Ok(None);
                    };
                    Ok(Some(name))
                })?
                .collect::<Result<Vec<Option<String>>, rusqlite::Error>>()
            })
            .collect::<Result<Vec<_>, _>>()?;
        let uniqueness: Vec<String> = uniqueness
            .into_iter()
            .flat_map(|v| v.into_iter().filter_map(|n| n))
            .collect();
        Ok(Self {
            columns: table_columns
                .into_iter()
                .map(|c| {
                    let is_unique = c.pk == 0 && uniqueness.contains(&c.name);
                    SqlColumn {
                        r#type: SqlColumnType::from_table_info(&c),
                        is_unique,
                        name: c.name.into(),
                        is_primary: c.pk != 0,
                    }
                })
                .collect(),
        })
    }

    fn from_definition<'a, T: ToTable<'a>>() -> Self {
        Self {
            columns: T::columns(None, false, false),
        }
    }
}
