use std::borrow::Cow;

trait ToFilter {
    fn to_filter(self) -> GenericFilter;
}

pub enum GenericFilter {
    None,
    And(Vec<GenericFilter>),
    Or(Vec<GenericFilter>),
    Not(Box<GenericFilter>),
    Field(FieldFilter),
}

pub struct FieldFilter {
    field: Cow<'static, str>,
    value: String,
    operator: FilterOperator,
}

pub enum FilterOperator {
    Equals,
    NotEquals,
    LessThan,
    LessThanEquals,
    GreaterThan,
    GreaterThanEquals,
    Like,
    Glob,
}
