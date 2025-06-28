use structured_sql::{Database, IntoSqlTable, SqlTable};

#[derive(Debug, IntoSqlTable)]
struct Test {
    #[silo(primary)]
    id: u32,
    value1: Point,
    value2: String,
    value3: FruitWithData,
}

#[derive(Debug, IntoSqlTable, Clone)]
struct Point {
    x: i32,
    y: i32,
}

#[derive(Debug, IntoSqlTable, Clone, Default)]
enum Fruit {
    #[default]
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

#[derive(Debug, Clone, IntoSqlTable)]
pub enum Availability {
    Now {
        player_url: String,
        video_url: VideoUrl,
    },
    Later,
}

#[derive(Debug, Clone, IntoSqlTable)]
pub enum VideoUrl {
    Direct(String),
    Blob(String),
}

#[derive(Debug, Eq, PartialEq, IntoSqlTable)]
pub struct Credits {
    #[silo(primary)]
    id: u32,
    cast_id: u32,
    crew_id: u32,
}

#[derive(Debug, Eq, PartialEq, IntoSqlTable)]
pub struct Crew {
    // credit_id: [u8; 12],
    department: String,
    gender: Option<u8>,
    id: u32,
    job: String,
    name: String,
    profile_path: Option<String>,
}

#[derive(Debug, Eq, PartialEq, IntoSqlTable)]
pub struct Cast {
    id: u32,
    cast_id: u32,
    // credit_id: [u8; 12],
    character: String,
    gender: Option<u8>,
    name: String,
    profile_path: Option<String>,
    order: u8,
}

#[derive(Debug, PartialEq, IntoSqlTable)]
pub struct Genre {
    #[silo(primary)]
    id: u16,
    name: String,
}

#[derive(Debug, PartialEq, IntoSqlTable)]
pub struct MovieWithGenres {
    movie_id: u32,
    genre_id: u16,
}

#[derive(Debug, PartialEq, IntoSqlTable)]
pub struct TmdbMovie {
    #[silo(primary)]
    id: u32,
    imdb_id: u32,
    title: String,
    tagline: String,
    original_title: String,
    original_language: String,
    overview: Option<String>,
    // #[silo(skip)]
    // release_date: time::Date,
    runtime: u32,
    homepage: Option<String>,
    // genres: Vec<Genre>,
    poster_path: Option<String>,
    backdrop_path: Option<String>,
    popularity: f64,
    budget: u64,
    adult: bool,
    credits: Option<Credits>,
}

const _: () = const { assert!(matches!(Point::COLUMNS.len(), 2)) };
const _: () = const { assert!(matches!(Test::COLUMNS.len(), 7)) };
const _: () = const { assert!(matches!(Fruit::COLUMNS.len(), 1)) };
const _: () = const { assert!(matches!(FruitWithData::COLUMNS.len(), 3)) };

fn main() {
    let test_db = Database::create_in_memory().unwrap();
    let test = test_db.load::<Test>().unwrap();
    test_db.save("test-before.db").unwrap();

    // dbg!(test);
    test.insert(Test {
        id: 0,
        value1: Point { x: 12, y: 42 },
        value2: "Hello".into(),
        value3: FruitWithData::Banana {
            ripeness: "Very".into(),
        },
    })
    .unwrap();
    let f = TestFilter {
        value1: (PointFilter {
            x: structured_sql::SqlColumnFilter::MustBeEqual(12),
            ..Default::default()
        }),
        // value3: (FruitWithDataFilter {
        //     filter: structured_sql::SqlColumnFilter::MustBeEqual("Banana"),
        // }),
        ..Default::default()
    };
    _ = dbg!(test.filter(f));
    test_db.save("test.db").unwrap();
    println!("Hello, world!");
}
