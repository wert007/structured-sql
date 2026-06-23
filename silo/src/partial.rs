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

impl<T: HasPartial> HasPartial for Option<T> {
    type Partial = Option<Option<T>>;
}
