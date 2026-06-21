struct PersonTable<'a> {
    connection: &'a silo::rusqlite::Connection,
}
impl<'a> PersonTable<'a> {
    fn default_order() -> silo::GenericOrder {
        let mut result = silo::GenericOrder::default();
        result
    }
}
impl<'a> silo::SqlTable<'a> for PersonTable<'a> {
    type RowType = Person;
    type ValueType = Person;
    type FilterType = PersonFilter;
    const INSERT_FAILURE_BEHAVIOR: silo::SqlFailureBehavior = silo::SqlFailureBehavior::Abort;
    fn connection(&self) -> &'a silo::rusqlite::Connection {
        self.connection
    }
    fn insert(&self, row: Self::RowType) -> Result<(), silo::rusqlite::Error> {
        silo::insert_into_table(&self.connection, row, Self::INSERT_FAILURE_BEHAVIOR)?;
        Ok(())
    }
    fn from_connection(connection: &'a silo::rusqlite::Connection) -> Self {
        Self { connection }
    }
}
impl<'a> silo::ToTable<'a> for Person {
    type Table = PersonTable<'a>;
    const NAME: &'static str = stringify!(Person);
    fn fill_columns(columns: &mut Vec<silo::SqlColumn>) {
        columns.extend([silo::SqlColumn {
            name: "name".into(),
            is_primary: false,
            is_unique: false,
            r#type: silo::SqlColumnType::Text,
        }]);
        columns.extend([silo::SqlColumn {
            name: "age".into(),
            is_primary: false,
            is_unique: false,
            r#type: silo::SqlColumnType::Integer,
        }]);
        columns.extend(
            <AddressTC as silo::ToColumns>::columns()
                .into_iter()
                .map(|mut c| {
                    c.name = format!("{}_{}", "residence", &c.name).into();
                    c
                }),
        );
    }
    fn insert_foreign_references(
        self,
        connection: &silo::rusqlite::Connection,
    ) -> Result<(), silo::rusqlite::Error> {
        use silo::AsForeignReference;
        self.name.insert_as_foreign_reference(connection)?;
        self.age.insert_as_foreign_reference(connection)?;
        self.residence.insert_as_foreign_reference(connection)?;
        Ok(())
    }
}
impl silo::MigrationHandler for Person {}
impl silo::FromRow for Person {
    fn try_from_row(
        row: &silo::rusqlite::Row,
        connection: &silo::rusqlite::Connection,
    ) -> Result<Self, silo::Error> {
        use silo::PartialType;
        PartialPerson::try_from_row(row, connection)?
            .transpose()
            .ok_or(silo::Error::Todo(
                "Improve error handling here, so we know which column was missing".into(),
            ))
    }
}
impl silo::FromRow for PartialPerson {
    fn try_from_row(
        row: &silo::rusqlite::Row,
        connection: &silo::rusqlite::Connection,
    ) -> Result<Self, silo::Error> {
        let name = <<String as silo::HasPartial>::Partial as silo::ExtractFromRow>::try_from_row(
            stringify!(name),
            row,
            connection,
        )?;
        let age = <<u8 as silo::HasPartial>::Partial as silo::ExtractFromRow>::try_from_row(
            stringify!(age),
            row,
            connection,
        )?;
        let residence =
            <<AddressTC as silo::HasPartial>::Partial as silo::ExtractFromRow>::try_from_row(
                stringify!(residence),
                row,
                connection,
            )?;
        Ok(Self {
            name,
            age,
            residence,
        })
    }
}
#[derive(Default)]
struct PartialPerson {
    name: <String as silo::HasPartial>::Partial,
    age: <u8 as silo::HasPartial>::Partial,
    residence: <AddressTC as silo::HasPartial>::Partial,
}
impl silo::PartialType<Person> for PartialPerson {
    fn transpose(self) -> Option<Person> {
        use silo::PartialType;
        let name = self.name.transpose()?;
        let age = self.age.transpose()?;
        let residence = self.residence.transpose()?;
        Some(Person {
            name,
            age,
            residence,
        })
    }
}
impl silo::HasPartial for Person {
    type Partial = PartialPerson;
}
impl Into<PartialPerson> for Person {
    fn into(self) -> PartialPerson {
        PartialPerson {
            name: self.name.into(),
            age: self.age.into(),
            residence: self.residence.into(),
        }
    }
}
impl silo::PartialRow for PartialPerson {
    fn used_column_names(&self, column_name: Option<String>) -> Vec<String> {
        let mut result = Vec::new();
        result.extend(
            self.name.used_column_names(Some(
                column_name
                    .clone()
                    .map(|c| format!("{c}_{}", stringify!(name)))
                    .unwrap_or_else(|| stringify!(name).to_string()),
            )),
        );
        result.extend(
            self.age.used_column_names(Some(
                column_name
                    .clone()
                    .map(|c| format!("{c}_{}", stringify!(age)))
                    .unwrap_or_else(|| stringify!(age).to_string()),
            )),
        );
        result.extend(
            self.residence.used_column_names(Some(
                column_name
                    .clone()
                    .map(|c| format!("{c}_{}", stringify!(residence)))
                    .unwrap_or_else(|| stringify!(residence).to_string()),
            )),
        );
        result
    }
    fn used_values(&self) -> Vec<&dyn silo::rusqlite::ToSql> {
        let mut result = Vec::new();
        result.extend(self.name.used_values());
        result.extend(self.age.used_values());
        result.extend(self.residence.used_values());
        result
    }
}
impl AsColumns for Person {
    const COLUMN_COUNT: usize = 0
        + <String as silo::AsParams>::COLUMN_COUNT
        + <u8 as silo::AsParams>::COLUMN_COUNT
        + <AddressTC as silo::AsParams>::COLUMN_COUNT;
    fn columns(parent: Option<&str>) -> Vec<SqlColumn> {
        let mut result = Vec::with_capacity(Self::COLUMN_COUNT);
        vec![SqlColumn {
            name: parent.unwrap().to_string().into(),
            r#type: T::SQL_COLUMN_TYPE,
            is_primary: false,
            is_unique: false,
        }]
    }
}
impl silo::TableAsParams for Person {
    fn as_params<'a>(&'a self) -> Vec<&'a dyn silo::rusqlite::ToSql> {
        use silo::AsParams;
        let mut result = Vec::with_capacity(<Self as silo::TableAsParams>::COLUMN_COUNT);
        result.extend(AsParams::as_params(&self.name));
        result.extend(AsParams::as_params(&self.age));
        result.extend(AsParams::as_params(&self.residence));
        result
    }
}
#[derive(Default)]
struct PersonFilter {
    generic: silo::filter::GenericFilter,
}
impl PersonFilter {
    fn or(&self, lhs: Self, rhs: Self) -> Self {
        Self {
            generic: silo::filter::GenericFilter::Or(vec![lhs.generic, rhs.generic]),
        }
    }
    fn and(&self, lhs: Self, rhs: Self) -> Self {
        Self {
            generic: silo::filter::GenericFilter::And(vec![lhs.generic, rhs.generic]),
        }
    }
    fn name_equals(&self, value: impl AsRef<str>) -> Self {
        let value = format!("\"{}\"", value.as_ref());
        Self {
            generic: silo::filter::GenericFilter::Field(silo::filter::FieldFilter {
                field: stringify!(name).into(),
                value,
                operator: silo::filter::FilterOperator::Equals,
            }),
        }
    }
    fn name_not_equals(&self, value: impl AsRef<str>) -> Self {
        let value = format!("\"{}\"", value.as_ref());
        Self {
            generic: silo::filter::GenericFilter::Field(silo::filter::FieldFilter {
                field: stringify!(name).into(),
                value,
                operator: silo::filter::FilterOperator::NotEquals,
            }),
        }
    }
    fn name_starts_with(&self, value: impl AsRef<str>) -> Self {
        let value = format!("\"{}%\"", value.as_ref());
        Self {
            generic: silo::filter::GenericFilter::Field(silo::filter::FieldFilter {
                field: stringify!(name).into(),
                value,
                operator: silo::filter::FilterOperator::Like,
            }),
        }
    }
    fn name_ends_with(&self, value: impl AsRef<str>) -> Self {
        let value = format!("\"%{}\"", value.as_ref());
        Self {
            generic: silo::filter::GenericFilter::Field(silo::filter::FieldFilter {
                field: stringify!(name).into(),
                value,
                operator: silo::filter::FilterOperator::Like,
            }),
        }
    }
    fn age_equals(&self, value: u8) -> Self {
        let value = format!("{}", value);
        Self {
            generic: silo::filter::GenericFilter::Field(silo::filter::FieldFilter {
                field: stringify!(age).into(),
                value,
                operator: silo::filter::FilterOperator::Equals,
            }),
        }
    }
    fn age_not_equals(&self, value: u8) -> Self {
        let value = format!("{}", value);
        Self {
            generic: silo::filter::GenericFilter::Field(silo::filter::FieldFilter {
                field: stringify!(age).into(),
                value,
                operator: silo::filter::FilterOperator::NotEquals,
            }),
        }
    }
    fn age_less_than(&self, value: u8) -> Self {
        let value = format!("{}", value);
        Self {
            generic: silo::filter::GenericFilter::Field(silo::filter::FieldFilter {
                field: stringify!(age).into(),
                value,
                operator: silo::filter::FilterOperator::LessThan,
            }),
        }
    }
    fn age_less_than_equals(&self, value: u8) -> Self {
        let value = format!("{}", value);
        Self {
            generic: silo::filter::GenericFilter::Field(silo::filter::FieldFilter {
                field: stringify!(age).into(),
                value,
                operator: silo::filter::FilterOperator::LessThanEquals,
            }),
        }
    }
    fn age_greater_than(&self, value: u8) -> Self {
        let value = format!("{}", value);
        Self {
            generic: silo::filter::GenericFilter::Field(silo::filter::FieldFilter {
                field: stringify!(age).into(),
                value,
                operator: silo::filter::FilterOperator::GreaterThan,
            }),
        }
    }
    fn age_greater_than_equals(&self, value: u8) -> Self {
        let value = format!("{}", value);
        Self {
            generic: silo::filter::GenericFilter::Field(silo::filter::FieldFilter {
                field: stringify!(age).into(),
                value,
                operator: silo::filter::FilterOperator::GreaterThanEquals,
            }),
        }
    }
}
impl silo::filter::ToFilter for PersonFilter {
    fn to_filter(self) -> silo::filter::GenericFilter {
        self.generic
    }
}
