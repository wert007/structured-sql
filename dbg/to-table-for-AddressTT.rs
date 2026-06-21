struct AddressTTTable<'a> {
    connection: &'a silo::rusqlite::Connection,
}
impl<'a> AddressTTTable<'a> {
    fn default_order() -> silo::GenericOrder {
        let mut result = silo::GenericOrder::default();
        result
    }
}
impl<'a> silo::SqlTable<'a> for AddressTTTable<'a> {
    type RowType = AddressTT;
    type ValueType = AddressTT;
    type FilterType = AddressTTFilter;
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
impl<'a> silo::ToTable<'a> for AddressTT {
    type Table = AddressTTTable<'a>;
    const NAME: &'static str = stringify!(AddressTT);
    fn fill_columns(columns: &mut Vec<silo::SqlColumn>) {
        columns.extend([silo::SqlColumn {
            name: "pk".into(),
            is_primary: true,
            is_unique: false,
            r#type: silo::SqlColumnType::Integer,
        }]);
        columns.extend([silo::SqlColumn {
            name: "city".into(),
            is_primary: false,
            is_unique: false,
            r#type: silo::SqlColumnType::Text,
        }]);
        columns.extend([silo::SqlColumn {
            name: "street".into(),
            is_primary: false,
            is_unique: false,
            r#type: silo::SqlColumnType::Text,
        }]);
    }
    fn insert_foreign_references(
        self,
        connection: &silo::rusqlite::Connection,
    ) -> Result<(), silo::rusqlite::Error> {
        use silo::AsForeignReference;
        self.pk.insert_as_foreign_reference(connection)?;
        self.city.insert_as_foreign_reference(connection)?;
        self.street.insert_as_foreign_reference(connection)?;
        Ok(())
    }
}
impl silo::MigrationHandler for AddressTT {}
impl silo::FromRow for AddressTT {
    fn try_from_row(
        row: &silo::rusqlite::Row,
        connection: &silo::rusqlite::Connection,
    ) -> Result<Self, silo::Error> {
        use silo::PartialType;
        PartialAddressTT::try_from_row(row, connection)?
            .transpose()
            .ok_or(silo::Error::Todo(
                "Improve error handling here, so we know which column was missing".into(),
            ))
    }
}
impl silo::ExtractFromRow for AddressTT {
    fn try_from_row_simple(column: &str, row: &silo::rusqlite::Row) -> Result<Self, silo::Error> {
        unreachable!("should not be called directly!")
    }
    fn try_from_row(
        column: &str,
        row: &silo::rusqlite::Row,
        connection: &silo::rusqlite::Connection,
    ) -> Result<Self, silo::Error> {
        use silo::PartialType;
        PartialAddressTT::try_from_row(column, row, connection)?
            .transpose()
            .ok_or(silo::Error::Todo(
                "Improve error handling here, so we know which column was missing".into(),
            ))
    }
}
impl silo::FromRow for PartialAddressTT {
    fn try_from_row(
        row: &silo::rusqlite::Row,
        connection: &silo::rusqlite::Connection,
    ) -> Result<Self, silo::Error> {
        let pk = <<u64 as silo::HasPartial>::Partial as silo::ExtractFromRow>::try_from_row(
            stringify!(pk),
            row,
            connection,
        )?;
        let city = <<String as silo::HasPartial>::Partial as silo::ExtractFromRow>::try_from_row(
            stringify!(city),
            row,
            connection,
        )?;
        let street = <<String as silo::HasPartial>::Partial as silo::ExtractFromRow>::try_from_row(
            stringify!(street),
            row,
            connection,
        )?;
        Ok(Self { pk, city, street })
    }
}
impl silo::ExtractFromRow for PartialAddressTT {
    fn try_from_row_simple(column: &str, row: &silo::rusqlite::Row) -> Result<Self, silo::Error> {
        unreachable!("should not be called directly!")
    }
    fn try_from_row(
        column: &str,
        row: &silo::rusqlite::Row,
        connection: &silo::rusqlite::Connection,
    ) -> Result<Self, silo::Error> {
        use silo::SqlTable;
        let pk: <u64 as silo::HasPartial>::Partial =
            row.get(format!("{}_{}", column, stringify!(pk)).as_str())?;
        let Some(pk) = pk else {
            return Ok(Default::default());
        };
        let __silo__db = unsafe { silo::Database::from_connection(connection) }?;
        let __silo__foreign = __silo__db.load::<AddressTT>()?;
        let mut results = __silo__foreign.load_where(|f| f.pk_equals(pk))?;
        assert_eq!(results.len(), 1, "Primary key was not unique!");
        Ok(results.pop().map(|r| r.into()).unwrap_or_default())
    }
}
#[derive(Default)]
struct PartialAddressTT {
    pk: <u64 as silo::HasPartial>::Partial,
    city: <String as silo::HasPartial>::Partial,
    street: <String as silo::HasPartial>::Partial,
}
impl silo::PartialType<AddressTT> for PartialAddressTT {
    fn transpose(self) -> Option<AddressTT> {
        use silo::PartialType;
        let pk = self.pk.transpose()?;
        let city = self.city.transpose()?;
        let street = self.street.transpose()?;
        Some(AddressTT { pk, city, street })
    }
}
impl silo::HasPartial for AddressTT {
    type Partial = PartialAddressTT;
}
impl Into<PartialAddressTT> for AddressTT {
    fn into(self) -> PartialAddressTT {
        PartialAddressTT {
            pk: self.pk.into(),
            city: self.city.into(),
            street: self.street.into(),
        }
    }
}
impl silo::PartialRow for PartialAddressTT {
    fn used_column_names(&self, column_name: Option<String>) -> Vec<String> {
        let mut result = Vec::new();
        result.extend(
            self.pk.used_column_names(Some(
                column_name
                    .clone()
                    .map(|c| format!("{c}_{}", stringify!(pk)))
                    .unwrap_or_else(|| stringify!(pk).to_string()),
            )),
        );
        result.extend(
            self.city.used_column_names(Some(
                column_name
                    .clone()
                    .map(|c| format!("{c}_{}", stringify!(city)))
                    .unwrap_or_else(|| stringify!(city).to_string()),
            )),
        );
        result.extend(
            self.street.used_column_names(Some(
                column_name
                    .clone()
                    .map(|c| format!("{c}_{}", stringify!(street)))
                    .unwrap_or_else(|| stringify!(street).to_string()),
            )),
        );
        result
    }
    fn used_values(&self) -> Vec<&dyn silo::rusqlite::ToSql> {
        let mut result = Vec::new();
        result.extend(self.pk.used_values());
        result.extend(self.city.used_values());
        result.extend(self.street.used_values());
        result
    }
}
impl AsColumns for AddressTT {
    const COLUMN_COUNT: usize = 0
        + <u64 as silo::AsParams>::COLUMN_COUNT
        + <String as silo::AsParams>::COLUMN_COUNT
        + <String as silo::AsParams>::COLUMN_COUNT;
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
impl silo::TableAsParams for AddressTT {
    fn as_params<'a>(&'a self) -> Vec<&'a dyn silo::rusqlite::ToSql> {
        use silo::AsParams;
        let mut result = Vec::with_capacity(<Self as silo::TableAsParams>::COLUMN_COUNT);
        result.extend(AsParams::as_params(&self.pk));
        result.extend(AsParams::as_params(&self.city));
        result.extend(AsParams::as_params(&self.street));
        result
    }
}
impl silo::AsForeignReference for AddressTT {
    fn insert_as_foreign_reference(
        self,
        connection: &rusqlite::Connection,
    ) -> Result<(), rusqlite::Error> {
        use silo::SqlTable;
        let db = unsafe { silo::Database::from_connection(connection) }?;
        let table = db.load::<Self>()?;
        table.insert(self)?;
        Ok(())
    }
}
impl silo::AsParams for AddressTT {
    const COLUMN_COUNT: usize = 1;
    fn as_params<'a>(&'a self) -> Vec<&'a dyn silo::rusqlite::ToSql> {
        let mut result = Vec::with_capacity(Self::COLUMN_COUNT);
        result.extend(self.pk.as_params());
        result
    }
}
impl silo::ToColumns for AddressTT {
    fn fill_columns(columns: &mut Vec<silo::SqlColumn>) {
        columns.push(silo::SqlColumn {
            name: stringify!(pk).into(),
            r#type: silo::SqlColumnType::Integer,
            is_primary: false,
            is_unique: false,
        });
    }
}
#[derive(Default)]
struct AddressTTFilter {
    generic: silo::filter::GenericFilter,
}
impl AddressTTFilter {
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
    fn pk_equals(&self, value: u64) -> Self {
        let value = format!("{}", value);
        Self {
            generic: silo::filter::GenericFilter::Field(silo::filter::FieldFilter {
                field: stringify!(pk).into(),
                value,
                operator: silo::filter::FilterOperator::Equals,
            }),
        }
    }
    fn pk_not_equals(&self, value: u64) -> Self {
        let value = format!("{}", value);
        Self {
            generic: silo::filter::GenericFilter::Field(silo::filter::FieldFilter {
                field: stringify!(pk).into(),
                value,
                operator: silo::filter::FilterOperator::NotEquals,
            }),
        }
    }
    fn pk_less_than(&self, value: u64) -> Self {
        let value = format!("{}", value);
        Self {
            generic: silo::filter::GenericFilter::Field(silo::filter::FieldFilter {
                field: stringify!(pk).into(),
                value,
                operator: silo::filter::FilterOperator::LessThan,
            }),
        }
    }
    fn pk_less_than_equals(&self, value: u64) -> Self {
        let value = format!("{}", value);
        Self {
            generic: silo::filter::GenericFilter::Field(silo::filter::FieldFilter {
                field: stringify!(pk).into(),
                value,
                operator: silo::filter::FilterOperator::LessThanEquals,
            }),
        }
    }
    fn pk_greater_than(&self, value: u64) -> Self {
        let value = format!("{}", value);
        Self {
            generic: silo::filter::GenericFilter::Field(silo::filter::FieldFilter {
                field: stringify!(pk).into(),
                value,
                operator: silo::filter::FilterOperator::GreaterThan,
            }),
        }
    }
    fn pk_greater_than_equals(&self, value: u64) -> Self {
        let value = format!("{}", value);
        Self {
            generic: silo::filter::GenericFilter::Field(silo::filter::FieldFilter {
                field: stringify!(pk).into(),
                value,
                operator: silo::filter::FilterOperator::GreaterThanEquals,
            }),
        }
    }
    fn city_equals(&self, value: impl AsRef<str>) -> Self {
        let value = format!("\"{}\"", value.as_ref());
        Self {
            generic: silo::filter::GenericFilter::Field(silo::filter::FieldFilter {
                field: stringify!(city).into(),
                value,
                operator: silo::filter::FilterOperator::Equals,
            }),
        }
    }
    fn city_not_equals(&self, value: impl AsRef<str>) -> Self {
        let value = format!("\"{}\"", value.as_ref());
        Self {
            generic: silo::filter::GenericFilter::Field(silo::filter::FieldFilter {
                field: stringify!(city).into(),
                value,
                operator: silo::filter::FilterOperator::NotEquals,
            }),
        }
    }
    fn city_starts_with(&self, value: impl AsRef<str>) -> Self {
        let value = format!("\"{}%\"", value.as_ref());
        Self {
            generic: silo::filter::GenericFilter::Field(silo::filter::FieldFilter {
                field: stringify!(city).into(),
                value,
                operator: silo::filter::FilterOperator::Like,
            }),
        }
    }
    fn city_ends_with(&self, value: impl AsRef<str>) -> Self {
        let value = format!("\"%{}\"", value.as_ref());
        Self {
            generic: silo::filter::GenericFilter::Field(silo::filter::FieldFilter {
                field: stringify!(city).into(),
                value,
                operator: silo::filter::FilterOperator::Like,
            }),
        }
    }
    fn street_equals(&self, value: impl AsRef<str>) -> Self {
        let value = format!("\"{}\"", value.as_ref());
        Self {
            generic: silo::filter::GenericFilter::Field(silo::filter::FieldFilter {
                field: stringify!(street).into(),
                value,
                operator: silo::filter::FilterOperator::Equals,
            }),
        }
    }
    fn street_not_equals(&self, value: impl AsRef<str>) -> Self {
        let value = format!("\"{}\"", value.as_ref());
        Self {
            generic: silo::filter::GenericFilter::Field(silo::filter::FieldFilter {
                field: stringify!(street).into(),
                value,
                operator: silo::filter::FilterOperator::NotEquals,
            }),
        }
    }
    fn street_starts_with(&self, value: impl AsRef<str>) -> Self {
        let value = format!("\"{}%\"", value.as_ref());
        Self {
            generic: silo::filter::GenericFilter::Field(silo::filter::FieldFilter {
                field: stringify!(street).into(),
                value,
                operator: silo::filter::FilterOperator::Like,
            }),
        }
    }
    fn street_ends_with(&self, value: impl AsRef<str>) -> Self {
        let value = format!("\"%{}\"", value.as_ref());
        Self {
            generic: silo::filter::GenericFilter::Field(silo::filter::FieldFilter {
                field: stringify!(street).into(),
                value,
                operator: silo::filter::FilterOperator::Like,
            }),
        }
    }
}
impl silo::filter::ToFilter for AddressTTFilter {
    fn to_filter(self) -> silo::filter::GenericFilter {
        self.generic
    }
}
