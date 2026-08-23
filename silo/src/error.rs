use std::borrow::Cow;

use crate::partial;

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
    #[error("There is no variant named {0} on enum {1}")]
    UnknownEnumVariant(String, &'static str),
    #[error("IllFormattedColumn: {1} cannot be parsed into {0}: {2:?}")]
    IllFormattedColumn(
        Cow<'static, str>,
        String,
        Option<Box<dyn std::error::Error + Send + Sync>>,
    ),
}

impl From<partial::TransposeError> for Error {
    fn from(value: partial::TransposeError) -> Self {
        match value {
            partial::TransposeError::MissingValue => {
                Self::Todo("Is this even possible to reach?".into())
            }
            partial::TransposeError::MissingColumn(column) => Self::MissingColumn(column),
        }
    }
}

impl PartialEq for Error {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Rusqlite(l0), Self::Rusqlite(r0)) => l0 == r0,
            (Self::MissingColumn(l0), Self::MissingColumn(r0)) => l0 == r0,
            (Self::WrongColumnType(l0, l1), Self::WrongColumnType(r0, r1)) => l0 == r0 && l1 == r1,
            (Self::CouldNotMigrate(l0), Self::CouldNotMigrate(r0)) => l0 == r0,
            (Self::Todo(l0), Self::Todo(r0)) => l0 == r0,
            (Self::UnknownEnumVariant(l0, l1), Self::UnknownEnumVariant(r0, r1)) => {
                l0 == r0 && l1 == r1
            }
            (Self::IllFormattedColumn(l0, l1, _), Self::IllFormattedColumn(r0, r1, _)) => {
                l0 == r0 && l1 == r1
            }
            _ => false,
        }
    }
}
