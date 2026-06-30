use silo_derive::{ToColumns, ToTable};
use uuid::Uuid;

use crate::{
    self as silo, AsColumns, AsColumnsDynamicallySized, Database, SqlTable, column_name_of,
    filter::{FieldFilter, Filterable, OptionalFilter},
};

#[derive(Default, Debug, PartialEq, Eq, Clone, silo::derive::ToColumns)]
struct AddressTC {
    city: String,
    street: String,
}

#[derive(Default, PartialEq, Eq, Debug, Clone, silo::derive::ToTable)]
struct Person {
    name: String,
    age: u8,
    traditional_name: Option<String>,
    #[silo(primary)]
    id: Uuid,
    residence: AddressTC,
}

#[test]
fn test_person_filter() {
    let db = Database::create_in_memory().unwrap();
    let persons = db.load::<Person>().unwrap();

    let alice = Person {
        id: Uuid::NAMESPACE_X500,
        name: "Alice".into(),
        age: 25,
        traditional_name: Some("Alicia".into()),
        residence: AddressTC {
            city: "Berlin".into(),
            street: "Main St".into(),
        },
    };

    let bob = Person {
        id: Uuid::NAMESPACE_DNS,
        name: "Bob".into(),
        age: 17,
        traditional_name: None,
        residence: AddressTC {
            city: "Munich".into(),
            street: "King St".into(),
        },
    };

    let charlie = Person {
        id: Uuid::NAMESPACE_OID,
        name: "Charlie".into(),
        age: 42,
        traditional_name: Some("Charles".into()),
        residence: AddressTC {
            city: "Berlin".into(),
            street: "Second St".into(),
        },
    };

    persons.insert(alice.clone()).unwrap();
    persons.insert(bob.clone()).unwrap();
    persons.insert(charlie.clone()).unwrap();

    // Equality
    let loaded = persons
        .load_where(PersonFilter {
            age: FieldFilter::equals(25),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(loaded, vec![alice.clone()]);

    // Greater than
    let loaded = persons
        .load_where(PersonFilter {
            age: FieldFilter::greater_than(18),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(loaded.len(), 2);
    assert!(loaded.contains(&alice));
    assert!(loaded.contains(&charlie));

    // Greater than or equal
    let loaded = persons
        .load_where(PersonFilter {
            age: FieldFilter::greater_than_equals(25),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(loaded.len(), 2);

    // Less than
    let loaded = persons
        .load_where(PersonFilter {
            age: FieldFilter::less_than(18),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(loaded, vec![bob.clone()]);

    // Less than or equal
    let loaded = persons
        .load_where(PersonFilter {
            age: FieldFilter::less_than_equals(17),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(loaded, vec![bob.clone()]);

    // String equality
    let loaded = persons
        .load_where(PersonFilter {
            name: FieldFilter::equals("Alice"),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(loaded, vec![alice.clone()]);

    // Optional field
    let loaded = persons
        .load_where(PersonFilter {
            traditional_name: OptionalFilter::IsSomeAnd(FieldFilter::equals("Charles")),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(loaded, vec![charlie.clone()]);

    // UUID
    let loaded = persons
        .load_where(PersonFilter {
            id: bob.id.convert_to_equals_filter(),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(loaded, vec![bob.clone()]);

    // Nested field
    let loaded = persons
        .load_where(PersonFilter {
            residence: AddressTCFilter {
                city: FieldFilter::equals("Berlin"),
                ..Default::default()
            },
            ..Default::default()
        })
        .unwrap();
    assert_eq!(loaded.len(), 2);
    assert!(loaded.contains(&alice));
    assert!(loaded.contains(&charlie));

    // Multiple filters should be ANDed
    let loaded = persons
        .load_where(PersonFilter {
            age: FieldFilter::greater_than(18),
            residence: AddressTCFilter {
                city: FieldFilter::equals("Berlin"),
                ..Default::default()
            },
            ..Default::default()
        })
        .unwrap();
    assert_eq!(loaded.len(), 2);

    let loaded = persons
        .load_where(PersonFilter {
            age: FieldFilter::greater_than(30),
            residence: AddressTCFilter {
                city: FieldFilter::equals("Berlin"),
                ..Default::default()
            },
            ..Default::default()
        })
        .unwrap();
    assert_eq!(loaded, vec![charlie.clone()]);

    // No match
    let loaded = persons
        .load_where(PersonFilter {
            age: FieldFilter::less_than(10),
            ..Default::default()
        })
        .unwrap();
    assert!(loaded.is_empty());

    // Empty filter should return everything
    let loaded = persons.load_where(PersonFilter::default()).unwrap();
    assert_eq!(loaded.len(), 3);

    let loaded = persons
        .load_where(PersonFilter {
            name: alice.name.clone().convert_to_equals_filter(),
            age: alice.age.clone().convert_to_equals_filter(),
            traditional_name: alice.traditional_name.clone().convert_to_equals_filter(),
            id: alice.id.clone().convert_to_equals_filter(),
            residence: alice.residence.clone().convert_to_equals_filter(),
        })
        .unwrap();
    assert_eq!(loaded, [alice])
}
#[test]
fn update_person() {
    let db = Database::create_in_memory().unwrap();
    let persons = db.load::<Person>().unwrap();

    let id = Uuid::NAMESPACE_OID;

    let original = Person {
        id,
        name: "Charlie".into(),
        age: 42,
        traditional_name: Some("Charles".into()),
        residence: AddressTC {
            city: "Berlin".into(),
            street: "Second St".into(),
        },
    };

    persons.insert(original.clone()).unwrap();

    // Update every mutable field.
    let updated = persons
        .update(
            id,
            PartialPerson {
                id: None,
                name: Some("Chuck".into()),
                age: Some(43),
                traditional_name: Some(Some("Carl".into())),
                residence: PartialAddressTC {
                    city: Some("Munich".into()),
                    street: Some("Third St".into()),
                },
            },
        )
        .unwrap();

    assert_eq!(updated, 1);

    let loaded = persons.load_where(id).unwrap();

    assert_eq!(loaded.len(), 1);

    assert_eq!(
        loaded[0],
        Person {
            id,
            name: "Chuck".into(),
            age: 43,
            traditional_name: Some("Carl".into()),
            residence: AddressTC {
                city: "Munich".into(),
                street: "Third St".into(),
            },
        }
    );

    let count = persons
        .update(
            id,
            PartialPerson {
                traditional_name: Some(None),
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(count, 1);

    let loaded = persons.load_where(id).unwrap();

    assert_eq!(loaded.len(), 1);
    assert_eq!(
        loaded[0],
        Person {
            id,
            name: "Chuck".into(),
            age: 43,
            traditional_name: None,
            residence: AddressTC {
                city: "Munich".into(),
                street: "Third St".into(),
            },
        }
    );
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

    let db = Database::create_in_memory().unwrap();

    let original = TypeCoverage {
        id: Uuid::nil(),

        u8_: 1,
        u16_: 2,
        u32_: 3,
        u64_: 4,
        usize_: 5,

        i8_: -1,
        i16_: -2,
        i32_: -3,
        i64_: -4,
        isize_: -5,

        f32_: 1.5,
        f64_: 2.5,

        bool_: false,

        string_: String::new(),

        uuid: Uuid::NAMESPACE_URL,

        option_string: None,
        option_i32: None,
        option_bool: None,

        nested: Nested {
            city: String::new(),
            street: String::new(),
            number: 0,
            verified: false,
        },
    };

    let db = db.load::<TypeCoverage>().unwrap();
    db.insert(original.clone()).unwrap();
    let loaded = db.load_where(()).unwrap();

    assert_eq!(loaded[0], original);
}
