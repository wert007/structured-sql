use silo_derive::{ToColumns, ToTable};
use uuid::Uuid;

use crate::{
    self as silo, AsColumns, AsColumnsDynamicallySized, Database, SqlTable, column_name_of,
};

#[derive(Default, Debug, Clone, silo::derive::ToColumns)]
struct AddressTC {
    city: String,
    street: String,
}

#[derive(Default, Debug, Clone, silo::derive::ToTable)]
struct Person {
    name: String,
    age: u8,
    traditional_name: Option<String>,
    #[silo(primary)]
    id: Uuid,
    residence: AddressTC,
}

#[test]
fn creates_table_for_nested_struct() {
    let db = Database::create_in_memory().unwrap();

    db.load::<Person>().unwrap();

    let conn = &db.connection;

    let mut stmt = conn
        .prepare("SELECT sql FROM sqlite_master WHERE type='table' AND name='Person'")
        .unwrap();

    let tables: Vec<String> = stmt
        .query_map([], |row| row.get(0))
        .unwrap()
        .map(|r| r.unwrap())
        .collect();

    let sql = &tables[0];

    assert!(sql.contains("\"name\" TEXT"));
    assert!(sql.contains("\"age\" INTEGER"));
    assert!(sql.contains("\"traditional_name\" TEXT"));
    assert!(sql.contains("\"id\" TEXT PRIMARY KEY"));
    assert!(sql.contains("\"residence_city\" TEXT"));
    assert!(sql.contains("\"residence_street\" TEXT"));
}

#[test]
fn insert_and_load_person() {
    let db = Database::create_in_memory().unwrap();

    let db = db.load::<Person>().unwrap();
    let person = Person {
        id: Uuid::max(),
        name: "Alice".into(),
        age: 25,
        traditional_name: Some("Alicia".into()),
        residence: AddressTC {
            city: "Berlin".into(),
            street: "Main St".into(),
        },
    };

    db.insert(person.clone()).unwrap();

    let persons = db.load_where(()).unwrap();

    assert_eq!(persons.len(), 1);

    let loaded = &persons[0];

    assert_eq!(loaded.name, person.name);
    assert_eq!(loaded.age, person.age);
    assert_eq!(loaded.traditional_name, person.traditional_name);
    assert_eq!(loaded.residence.city, person.residence.city);
    assert_eq!(loaded.residence.street, person.residence.street);
}

#[test]
fn nested_columns_are_flattened() {
    use silo::AsColumnsDynamicallySized;
    let cols = Person::columns(None, false, false);

    assert_eq!(
        cols.iter().map(|c| &c.name).collect::<Vec<_>>(),
        vec![
            "name",
            "age",
            "traditional_name",
            "id",
            "residence_city",
            "residence_street",
        ]
    );
}

#[test]
fn test_3_level_deep_nesting() {
    #[derive(Debug, Clone, ToColumns)]
    struct Country {
        code: String,
    }

    #[derive(Debug, Clone, ToColumns)]
    struct Address {
        city: String,
        country: Country,
    }

    #[derive(Debug, Clone, ToTable)]
    struct Person {
        address: Address,
    }

    let c = column_name_of!(Person, address.country.code);
    assert_eq!(c, "address_country_code");
    let columns: Vec<_> = Person::columns(None, false, false)
        .into_iter()
        .map(|c| c.name)
        .collect();
    assert_eq!(columns, ["address_city", "address_country_code"]);
}

#[test]
fn test_3_level_deep_nesting_with_option() {
    #[derive(Debug, Clone, ToColumns)]
    struct Country {
        code: String,
    }

    #[derive(Debug, Clone, ToColumns)]
    struct Address {
        city: String,
        country: Country,
    }

    #[derive(Debug, Clone, ToTable)]
    struct Person {
        address: Option<Address>,
    }

    let c = column_name_of!(Person, address.country.code);
    assert_eq!(c, "address_country_code");
    let columns: Vec<_> = Person::columns(None, false, false)
        .into_iter()
        .map(|c| c.name)
        .collect();
    assert_eq!(columns, ["address_city", "address_country_code"]);
}

#[test]
fn duplicate_names() {
    #[derive(Debug, Clone, ToColumns)]
    struct A {
        city: String,
    }

    #[derive(Debug, Clone, ToColumns)]
    struct B {
        city: String,
    }

    #[derive(Debug, Clone, ToColumns)]
    struct C {
        a: A,
        b: B,
    }

    assert_eq!(column_name_of!(C, a.city), "a_city");
    assert_eq!(column_name_of!(C, b.city), "b_city");

    let columns: Vec<_> = C::columns(None, false, false)
        .into_iter()
        .map(|c| c.name)
        .collect();
    assert_eq!(columns, ["a_city", "b_city"]);
}

#[test]
fn test_rust_keywords_to_table() {
    #[derive(Debug, Clone, ToTable)]
    struct Foo {
        r#type: String,
    }
    assert_eq!(column_name_of!(Foo, r#type), "type");
    let columns: Vec<_> = Foo::columns(None, false, false)
        .into_iter()
        .map(|c| c.name)
        .collect();
    assert_eq!(columns, ["type"]);
}

#[test]
#[allow(non_camel_case_types)]
fn test_rust_keywords_as_table_name_to_table() {
    #[derive(Debug, Clone, ToTable)]
    struct r#for {
        r#type: String,
    }
    use silo::ToTable;
    assert_eq!(r#for::NAME, "for");
}

#[test]
fn test_rust_keywords_to_columns() {
    #[derive(Debug, Clone, ToColumns)]
    struct Foo {
        r#type: String,
    }
}

#[test]
fn test_sqlite_keywords_to_table() {
    #[derive(Debug, Clone, ToTable, PartialEq)]
    struct Foo {
        values: String,
    }

    let db = Database::create_in_memory().unwrap();
    let foo_table = db.load::<Foo>().unwrap();
    let og = Foo {
        values: "lkdjasda".into(),
    };
    foo_table.insert(og.clone()).unwrap();
    let loaded = foo_table.load_where(()).unwrap();
    assert_eq!(loaded.len(), 1);
    assert_eq!(og, loaded[0]);
}

#[test]
fn test_sqlite_keywords_as_table_names_to_table() {
    #[derive(Debug, Clone, ToTable, PartialEq)]
    struct Values {
        values: String,
    }

    let db = Database::create_in_memory().unwrap();
    let foo_table = db.load::<Values>().unwrap();
    let og = Values {
        values: "lkdjasda".into(),
    };
    foo_table.insert(og.clone()).unwrap();
    let loaded = foo_table.load_where(()).unwrap();
    assert_eq!(loaded.len(), 1);
    assert_eq!(og, loaded[0]);
}

#[test]
fn test_sqlite_keywords_to_columns() {
    #[derive(Debug, Clone, ToColumns)]
    struct Foo {
        values: String,
    }
}

#[test]
fn all_builtin_types_are_supported() {
    fn assert_supported<T: AsColumns>() {}
    assert_supported::<bool>();

    assert_supported::<u8>();
    assert_supported::<u16>();
    assert_supported::<u32>();
    assert_supported::<u64>();
    assert_supported::<usize>();

    assert_supported::<i8>();
    assert_supported::<i16>();
    assert_supported::<i32>();
    assert_supported::<i64>();
    assert_supported::<isize>();

    assert_supported::<f32>();
    assert_supported::<f64>();

    assert_supported::<String>();
    assert_supported::<Uuid>();

    assert_supported::<Option<i32>>();
    assert_supported::<Option<String>>();
}

#[test]
fn roundtrip_serialization() {
    #[derive(Debug, Clone, PartialEq, silo::derive::ToColumns)]
    struct Nested {
        city: String,
        street: String,
        number: u16,
        verified: bool,
    }

    #[derive(Debug, Clone, PartialEq, silo::derive::ToTable)]
    struct TypeCoverage {
        #[silo(primary)]
        id: Uuid,

        // integers
        u8_: u8,
        u16_: u16,
        u32_: u32,
        u64_: u64,
        usize_: usize,

        i8_: i8,
        i16_: i16,
        i32_: i32,
        i64_: i64,
        isize_: isize,

        // floating point
        f32_: f32,
        f64_: f64,

        // misc
        bool_: bool,
        string_: String,
        uuid: Uuid,

        // nullable
        option_string: Option<String>,
        option_i32: Option<i32>,
        option_bool: Option<bool>,

        // nested object
        nested: Nested,
    }

    let db = Database::create_in_memory().unwrap();

    let original = TypeCoverage {
        id: Uuid::max(),

        u8_: u8::MAX,
        u16_: u16::MAX,
        u32_: u32::MAX,
        u64_: u64::MAX,
        usize_: 123456,

        i8_: i8::MIN,
        i16_: i16::MIN,
        i32_: i32::MIN,
        i64_: i64::MIN,
        isize_: -123456,

        f32_: std::f32::consts::PI,
        f64_: std::f64::consts::E,

        bool_: true,

        string_: "Hello, 世界 🌍".to_owned(),

        uuid: Uuid::max(),

        option_string: Some("optional".into()),
        option_i32: Some(-42),
        option_bool: Some(false),

        nested: Nested {
            city: "Berlin".into(),
            street: "Unter den Linden".into(),
            number: 42,
            verified: true,
        },
    };

    let db = db.load::<TypeCoverage>().unwrap();
    db.insert(original.clone()).unwrap();
    let loaded = db.load_where(()).unwrap();

    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0], original);
}
