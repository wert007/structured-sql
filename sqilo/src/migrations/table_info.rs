use crate::{
    AsColumns, AsColumnsDynamicallySized, AsColumnsOptional, AsParams, AsParamsOptional, Error,
    ExtractFromRow, FromRow, SqlColumn, SqlTable, ToSqlDyn, ToTable, filter, partial, update,
};

#[derive(Debug, Clone)]
pub struct TableInfo {
    pub name: String,
    pub r#type: String,
    pub notnull: bool,
    pub pk: u32,
}

pub struct TableInfoTable<'__silo__a> {
    connection: &'__silo__a rusqlite::Connection,
    _marker: (),
}
impl<'__silo__a> SqlTable<'__silo__a> for TableInfoTable<'__silo__a> {
    type RowType = TableInfo;
    type ValueType = TableInfo;
    type FilterType = TableInfoFilter;
    fn connection(&self) -> &'__silo__a rusqlite::Connection {
        self.connection
    }
    fn update(
        &self,
        filter: impl Into<Self::FilterType>,
        updated: PartialTableInfo,
    ) -> std::result::Result<usize, rusqlite::Error> {
        use partial::PartialType;
        if updated.is_empty() {
            return Ok(0);
        }
        update::<TableInfo, PartialTableInfo, Self::FilterType>(&self.connection, filter, updated)
    }
    fn from_connection(connection: &'__silo__a rusqlite::Connection) -> Self {
        Self {
            connection,
            _marker: Default::default(),
        }
    }
}
impl<'__silo__a> ToTable<'__silo__a> for TableInfo {
    type Table = TableInfoTable<'__silo__a>;
    fn table_name() -> std::borrow::Cow<'static, str> {
        "TableInfo".into()
    }
}
impl FromRow for TableInfo {
    fn try_from_row(
        row: &rusqlite::Row,
        connection: &rusqlite::Connection,
    ) -> std::result::Result<Self, Error> {
        use partial::PartialType;
        Ok(PartialTableInfo::try_from_row(row, connection)?.transpose()?)
    }
}
impl FromRow for PartialTableInfo {
    fn try_from_row(
        row: &rusqlite::Row,
        connection: &rusqlite::Connection,
    ) -> std::result::Result<Self, Error> {
        let name = <<String as partial::HasPartial>::Partial as ExtractFromRow>::try_from_row(
            "name", row, connection,
        )?;
        let r#type = <<String as partial::HasPartial>::Partial as ExtractFromRow>::try_from_row(
            "type", row, connection,
        )?;
        let notnull = <<bool as partial::HasPartial>::Partial as ExtractFromRow>::try_from_row(
            "notnull", row, connection,
        )?;
        let pk = <<u32 as partial::HasPartial>::Partial as ExtractFromRow>::try_from_row(
            "pk", row, connection,
        )?;
        Ok(Self {
            name,
            r#type,
            notnull,
            pk,
        })
    }
}
pub struct PartialTableInfo {
    name: <String as partial::HasPartial>::Partial,
    r#type: <String as partial::HasPartial>::Partial,
    notnull: <bool as partial::HasPartial>::Partial,
    pk: <u32 as partial::HasPartial>::Partial,
}
impl Default for PartialTableInfo {
    fn default() -> Self {
        Self {
            name: Default::default(),
            r#type: Default::default(),
            notnull: Default::default(),
            pk: Default::default(),
        }
    }
}
impl partial::PartialType<TableInfo> for PartialTableInfo {
    fn is_empty(&self) -> bool {
        self.name.is_empty()
            && self.r#type.is_empty()
            && self.notnull.is_empty()
            && self.pk.is_empty()
            && true
    }
    fn transpose(self) -> Result<TableInfo, partial::TransposeError> {
        use partial::PartialType;
        let name = self
            .name
            .transpose()
            .map_err(|e| e.supply_field_name(stringify!(name)))?;
        let r#type = self
            .r#type
            .transpose()
            .map_err(|e| e.supply_field_name(stringify!(r#type)))?;
        let notnull = self
            .notnull
            .transpose()
            .map_err(|e| e.supply_field_name(stringify!(notnull)))?;
        let pk = self
            .pk
            .transpose()
            .map_err(|e| e.supply_field_name(stringify!(pk)))?;
        Ok(TableInfo {
            name,
            r#type,
            notnull,
            pk,
        })
    }
}
impl partial::HasPartial for TableInfo {
    type Partial = PartialTableInfo;
}
impl AsColumnsOptional for PartialTableInfo {
    fn columns_skip_optional(
        &self,
        parent: Option<&str>,
        is_unique: bool,
        is_primary: bool,
    ) -> Vec<SqlColumn> {
        let parent = parent.map(|p| format!("{p}_")).unwrap_or_default();
        let mut result = Vec::new();
        result.append(&mut self.name.columns_skip_optional(
            Some(&format!("{parent}{}", "name")),
            false,
            false,
        ));
        result.append(&mut self.r#type.columns_skip_optional(
            Some(&format!("{parent}{}", "type")),
            false,
            false,
        ));
        result.append(&mut self.notnull.columns_skip_optional(
            Some(&format!("{parent}{}", "notnull")),
            false,
            false,
        ));
        result.append(&mut self.pk.columns_skip_optional(
            Some(&format!("{parent}{}", "pk")),
            false,
            false,
        ));
        result
    }
}
impl AsParamsOptional for PartialTableInfo {
    fn as_params_skip_optional<'b>(&'b self) -> Vec<ToSqlDyn<'b>> {
        let mut result = Vec::new();
        result.append(&mut self.name.as_params_skip_optional());
        result.append(&mut self.r#type.as_params_skip_optional());
        result.append(&mut self.notnull.as_params_skip_optional());
        result.append(&mut self.pk.as_params_skip_optional());
        result
    }
}
impl Into<PartialTableInfo> for TableInfo {
    fn into(self) -> PartialTableInfo {
        PartialTableInfo {
            name: self.name.into(),
            r#type: self.r#type.into(),
            notnull: self.notnull.into(),
            pk: self.pk.into(),
        }
    }
}
impl AsColumns for TableInfo {
    const COLUMN_COUNT: usize = 0
        + <String as AsColumns>::COLUMN_COUNT
        + <String as AsColumns>::COLUMN_COUNT
        + <bool as AsColumns>::COLUMN_COUNT
        + <u32 as AsColumns>::COLUMN_COUNT;
}
impl AsColumnsDynamicallySized for TableInfo {
    fn columns(parent: Option<&str>, is_unique: bool, is_primary: bool) -> Vec<SqlColumn> {
        assert!(!is_unique);
        assert!(!is_primary);
        let parent = parent.map(|p| format!("{p}_")).unwrap_or_default();
        let mut result = Vec::with_capacity(<Self as AsColumns>::COLUMN_COUNT);
        result.append(&mut <String as AsColumnsDynamicallySized>::columns(
            Some(&format!("{parent}{}", "name")),
            false,
            false,
        ));
        result.append(&mut <String as AsColumnsDynamicallySized>::columns(
            Some(&format!("{parent}{}", "type")),
            false,
            false,
        ));
        result.append(&mut <bool as AsColumnsDynamicallySized>::columns(
            Some(&format!("{parent}{}", "notnull")),
            false,
            false,
        ));
        result.append(&mut <u32 as AsColumnsDynamicallySized>::columns(
            Some(&format!("{parent}{}", "pk")),
            false,
            false,
        ));
        result
    }
}
impl AsParams for TableInfo {
    fn as_params<'a>(&'a self) -> Vec<ToSqlDyn<'a>> {
        use AsParams;
        let mut result = Vec::with_capacity(<Self as AsColumns>::COLUMN_COUNT);
        result.extend(AsParams::as_params(&self.name));
        result.extend(AsParams::as_params(&self.r#type));
        result.extend(AsParams::as_params(&self.notnull));
        result.extend(AsParams::as_params(&self.pk));
        result
    }
}
pub struct TableInfoFilter {
    pub name: <String as filter::Filterable>::Filter,
    pub r#type: <String as filter::Filterable>::Filter,
    pub notnull: <bool as filter::Filterable>::Filter,
    pub pk: <u32 as filter::Filterable>::Filter,
}
impl Default for TableInfoFilter {
    fn default() -> Self {
        Self {
            name: Default::default(),
            r#type: Default::default(),
            notnull: Default::default(),
            pk: Default::default(),
        }
    }
}
impl From<()> for TableInfoFilter {
    fn from((): ()) -> Self {
        Self::default()
    }
}
impl filter::Filter for TableInfoFilter {
    fn to_sql(&self, sql: &mut String, parent: Option<&str>) {
        let parent = parent.map(|p| format!("{p}_")).unwrap_or_default();
        self.name.to_sql(sql, Some(&format!("{parent}{}", "name")));
        self.r#type
            .to_sql(sql, Some(&format!("{parent}{}", "type")));
        self.notnull
            .to_sql(sql, Some(&format!("{parent}{}", "notnull")));
        self.pk.to_sql(sql, Some(&format!("{parent}{}", "pk")));
    }

    fn not(self) -> Self {
        todo!()
    }

    fn or(self, other: Self) -> Self {
        todo!()
    }

    fn and(self, other: Self) -> Self {
        todo!()
    }
}
impl AsParams for TableInfoFilter {
    fn as_params<'a>(&'a self) -> Vec<ToSqlDyn<'a>> {
        use AsParams;
        let mut result = Vec::new();
        result.extend(AsParams::as_params(&self.name));
        result.extend(AsParams::as_params(&self.r#type));
        result.extend(AsParams::as_params(&self.notnull));
        result.extend(AsParams::as_params(&self.pk));
        result
    }
}
impl filter::Filterable for TableInfo {
    type Filter = TableInfoFilter;
    fn convert_to_equals_filter(self) -> Self::Filter {
        Self::Filter {
            name: self.name.convert_to_equals_filter(),
            r#type: self.r#type.convert_to_equals_filter(),
            notnull: self.notnull.convert_to_equals_filter(),
            pk: self.pk.convert_to_equals_filter(),
        }
    }
}
