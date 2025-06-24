use structured_sql::{Database, IntoSqlTable, SqlTable};

#[derive(Debug, IntoSqlTable)]
struct Test {
    value1: Point,
    value2: String,
    value3: FruitWithData,
}

#[derive(Debug, IntoSqlTable, Clone)]
struct Point {
    x: i32,
    y: i32,
}

#[derive(Debug, IntoSqlTable, Clone)]
enum Fruit {
    Apple,
    Pear,
    Banana,
    Strawberry,
}

#[derive(Debug, IntoSqlTable, Clone)]
enum FruitWithData {
    Apple(f32),
    Pear,
    Banana { ripeness: String },
}

const _: () = const { assert!(matches!(Point::COLUMNS.len(), 2)) };
const _: () = const { assert!(matches!(Test::COLUMNS.len(), 6)) };
const _: () = const { assert!(matches!(FruitWithData::COLUMNS.len(), 3)) };

fn main() {
    let test_db = Database::create_in_memory().unwrap();
    dbg!(Test::COLUMNS);
    let test = test_db.load::<Test>().unwrap();
    // dbg!(test);
    test.insert(Test {
        value1: Point { x: 12, y: 42 },
        value2: "Hello".into(),
        value3: FruitWithData::Banana {
            ripeness: "Very".into(),
        },
    })
    .unwrap();
    let f = TestFilter {
        value1: Some(structured_sql::SqlColumnFilter::MustBeEqual(PointFilter {
            x: Some(structured_sql::SqlColumnFilter::MustBeEqual(12)),
            ..Default::default()
        })),
        value3: Some(structured_sql::SqlColumnFilter::MustBeEqual(
            FruitWithDataFilter {
                filter: structured_sql::SqlColumnFilter::MustBeEqual("Banana"),
            },
        )),
        ..Default::default()
    };
    _ = dbg!(test.filter(f));
    test_db.save("test.db").unwrap();
    println!("Hello, world!");
}
