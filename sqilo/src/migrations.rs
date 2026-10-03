mod table_info;

use crate::{Database, FromRow, SqlColumn, SqlColumnType, SqlTable, ToTable};
use rusqlite::Connection;
pub use table_info::TableInfo;

pub trait Migrateable {
    fn migrate_each(
        db: &Database,
        migration_behavior: MigrationBehavior,
    ) -> Result<(), crate::error::Error>;
}

impl<M: for<'a> ToTable<'a>> Migrateable for M {
    fn migrate_each(
        db: &Database,
        migration_behavior: MigrationBehavior,
    ) -> Result<(), crate::error::Error> {
        db.load_and_migrate::<Self>(migration_behavior)?;
        Ok(())
    }
}

macro_rules! impl_migrateable_tuples {
    ($($t:ident),+$(,)?) => {
        impl<$($t: Migrateable,)+> Migrateable for ($($t,)+) {
            fn migrate_each(
                db: &Database,
                migration_behavior: MigrationBehavior,
            ) -> Result<(), crate::error::Error> {
                $(
                    $t::migrate_each(db, migration_behavior)?;
                )*
                Ok(())
            }
        }
    };
}

impl_migrateable_tuples!(T1, T2);
impl_migrateable_tuples!(T1, T2, T3);
impl_migrateable_tuples!(T1, T2, T3, T4);
impl_migrateable_tuples!(T1, T2, T3, T4, T5);
impl_migrateable_tuples!(T1, T2, T3, T4, T5, T6,);
impl_migrateable_tuples!(T1, T2, T3, T4, T5, T6, T7);
impl_migrateable_tuples!(T1, T2, T3, T4, T5, T6, T7, T8);
impl_migrateable_tuples!(T1, T2, T3, T4, T5, T6, T7, T8, T9);
impl_migrateable_tuples!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10);
impl_migrateable_tuples!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11);
impl_migrateable_tuples!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12);
impl_migrateable_tuples!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13);
impl_migrateable_tuples!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14);
impl_migrateable_tuples!(
    T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15
);
impl_migrateable_tuples!(
    T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16
);

impl Database {
    pub fn migrate_only<M: Migrateable>(
        &self,
        migration_behavior: MigrationBehavior,
    ) -> Result<(), crate::error::Error> {
        M::migrate_each(self, migration_behavior)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum MigrationError {
    #[error("Unexpected error occured: {0}")]
    Rusqlite(#[from] rusqlite::Error),
    #[error("Invalid changes have occured in table.")]
    InvalidChanges(SchemaDiff),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MigrationBehavior {
    MigrationHandler,
    #[default]
    NoMigration,
    AllowOnlyColumnDeletion,
    AllowOnlyColumnAddition,
    AllowColumnAdditionAndDeletion,
}

impl MigrationBehavior {
    fn work_through_diff(
        &self,
        diff: SchemaDiff,
        connection: &Connection,
        table_name: &str,
    ) -> Result<(), MigrationError> {
        match self {
            MigrationBehavior::MigrationHandler => return Ok(()),
            MigrationBehavior::NoMigration => {
                if !diff.is_empty() {
                    return Err(MigrationError::InvalidChanges(diff));
                }
            }
            MigrationBehavior::AllowOnlyColumnDeletion => {
                if !diff.added.is_empty() || !diff.retyped.is_empty() {
                    return Err(MigrationError::InvalidChanges(diff));
                }
            }
            MigrationBehavior::AllowOnlyColumnAddition => {
                if !diff.removed.is_empty() || !diff.retyped.is_empty() {
                    return Err(MigrationError::InvalidChanges(diff));
                }
            }
            MigrationBehavior::AllowColumnAdditionAndDeletion => {
                if !diff.retyped.is_empty() {
                    return Err(MigrationError::InvalidChanges(diff));
                }
            }
        }
        for added in diff.added {
            connection.execute(
                &format!(
                    "ALTER TABLE {table_name} ADD COLUMN {} {}",
                    added.name(),
                    added.r#type.as_sql(),
                ),
                (),
            )?;
        }
        for removed in diff.removed {
            connection.execute(
                &format!("ALTER TABLE {table_name} DROP COLUMN {}", removed.name(),),
                (),
            )?;
        }
        Ok(())
    }
}

impl From<crate::Error> for MigrationError {
    fn from(value: crate::Error) -> Self {
        todo!()
    }
}

pub fn check_table_schema<'a, T: ToTable<'a>>(
    table: &T::Table,
    behavior: MigrationBehavior,
) -> Result<(), MigrationError> {
    let existing_schema = Schema::from_connection(table.connection(), &T::table_name())?;
    let desired_schema = Schema::from_definition::<T>();
    let diff = existing_schema.diff_to(&desired_schema);
    if diff.is_empty() {
        return Ok(());
    }
    behavior.work_through_diff(diff, table.connection(), &T::table_name())?;
    Ok(())
}

#[derive(Debug)]
struct Schema {
    columns: Vec<SqlColumn>,
}

#[derive(Debug, Default)]
pub struct SchemaDiff {
    added: Vec<SqlColumn>,
    removed: Vec<SqlColumn>,
    retyped: Vec<(SqlColumn, SqlColumn)>,
}
impl SchemaDiff {
    fn is_empty(&self) -> bool {
        self.added.is_empty() && self.removed.is_empty() && self.retyped.is_empty()
    }
}

impl Schema {
    fn diff_to(&self, other: &Self) -> SchemaDiff {
        let mut diff = SchemaDiff::default();
        let mut self_columns = self.columns.clone();
        self_columns.sort_by_cached_key(|c| c.name.clone().into_owned());
        let mut other_columns = other.columns.clone();
        other_columns.sort_by_key(|c| c.name().to_string());
        let mut self_columns_iter = self_columns.iter().peekable();
        let mut other_columns_iter = other_columns.iter().peekable();
        loop {
            match (
                self_columns_iter.peek().copied(),
                other_columns_iter.peek().copied(),
            ) {
                (Some(s), Some(o)) if s.name() == o.name() => {
                    if s.r#type != o.r#type {
                        diff.retyped.push((s.clone(), o.clone()));
                    }
                    if s.is_unique != o.is_unique || s.is_primary != o.is_primary {
                        todo!()
                    }
                    self_columns_iter.next();
                    other_columns_iter.next();
                }
                (Some(s), Some(o)) => {
                    if s.name() < o.name() {
                        diff.removed.push(s.clone());
                        self_columns_iter.next();
                    } else {
                        diff.added.push(o.clone());
                        other_columns_iter.next();
                    }
                }
                (None, Some(o)) => {
                    diff.added.push(o.clone());
                    other_columns_iter.next();
                }
                (Some(s), None) => {
                    diff.removed.push(s.clone());
                    self_columns_iter.next();
                }
                (None, None) => return diff,
            }
        }
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
                    // Little quirk of sqilo, that primary keys are implied to be
                    // unique and as such do not have is_unique set to true.
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
