use std::borrow::Cow;

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
