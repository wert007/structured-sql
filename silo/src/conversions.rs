use time::{format_description::FormatItem, macros::format_description};

pub trait ToSqlValueString {
    fn to_sql_value_string(self) -> String;
}

impl ToSqlValueString for uuid::Uuid {
    fn to_sql_value_string(self) -> String {
        self.to_string()
    }
}

impl ToSqlValueString for uuid::NonNilUuid {
    fn to_sql_value_string(self) -> String {
        self.get().to_string()
    }
}

impl ToSqlValueString for time::Time {
    fn to_sql_value_string(self) -> String {
        const TIME_FORMAT: &[FormatItem<'_>] = format_description!(
            version = 2,
            "[hour]:[minute][optional [:[second][optional [.[subsecond]]]]]"
        );
        self.format(&TIME_FORMAT).unwrap()
    }
}

impl ToSqlValueString for time::Date {
    fn to_sql_value_string(self) -> String {
        const DATE_FORMAT: &[FormatItem<'_>] =
            format_description!(version = 2, "[year]-[month]-[day]");
        self.format(&DATE_FORMAT).unwrap()
    }
}

impl ToSqlValueString for time::OffsetDateTime {
    fn to_sql_value_string(self) -> String {
        const OFFSET_DATE_TIME_ENCODING: &[FormatItem<'_>] = format_description!(
            version = 2,
            "[year]-[month]-[day] [hour]:[minute]:[second].[subsecond][offset_hour sign:mandatory]:[offset_minute]"
        );
        self.format(&OFFSET_DATE_TIME_ENCODING).unwrap()
    }
}

impl ToSqlValueString for chrono::DateTime<chrono::Utc> {
    fn to_sql_value_string(self) -> String {
        self.to_rfc3339()
    }
}
