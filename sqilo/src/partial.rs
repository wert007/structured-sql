use std::borrow::Cow;

pub enum TransposeError {
    MissingValue,
    MissingColumn(Cow<'static, str>),
}

impl TransposeError {
    pub fn supply_field_name(self, field_name: &'static str) -> Self {
        match self {
            TransposeError::MissingValue => Self::MissingColumn(field_name.into()),
            TransposeError::MissingColumn(column) => {
                Self::MissingColumn([field_name, "_", &column].concat().into())
            }
        }
    }
}

pub trait PartialType<T> {
    fn is_empty(&self) -> bool;
    fn transpose(self) -> Result<T, TransposeError>;
}

impl<T> PartialType<T> for Option<T> {
    fn is_empty(&self) -> bool {
        self.is_none()
    }

    fn transpose(self) -> Result<T, TransposeError> {
        self.ok_or(TransposeError::MissingValue)
    }
}

impl<T: super::AsParams + super::ExtractFromRow + super::IsSingleColumn>
    super::SiloPartialMarkerTrait<T> for Option<T>
{
}

pub trait HasPartial<T = Self>: Sized + Into<Self::Partial> {
    // TODO: find out why we do not have partial type here!
    type Partial: Default + PartialType<Self>;
    // type Partial: PartialType<T>;
}

impl<T: HasPartial> HasPartial for Option<T> {
    type Partial = Option<Option<T>>;
}
