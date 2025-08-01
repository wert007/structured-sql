use silo::{Database, IntoSqlTable, MigrationHandler, PartialType, SqlTable, StaticStringStorage};

extern crate alloc;
extern crate core;
// mod crashtest;

#[derive(Debug, IntoSqlTable, Clone)]
#[silo(migrate)]
struct Point {
    x: i32,
    // #[silo(skip)]
    y: i32,
}

impl MigrationHandler for Point {}

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

#[derive(Debug, IntoSqlTable, Clone)]
#[silo(migrate)]
struct Test {
    #[silo(primary)]
    id: u32,
    value1: Point,
    // #[silo(skip)]
    // #[silo(unique)]
    value2: String,
    value3: FruitWithData,
    age: f64,
}

impl MigrationHandler for Test {
    fn migrate(
        string_storage: &mut StaticStringStorage,
        mut partial: Self::Partial,
        row: &silo::rusqlite::Row,
        connection: &silo::rusqlite::Connection,
    ) -> Option<Self> {
        use silo::FromRow;
        let age = u32::try_from_row(string_storage, Some("age"), row, connection).map(|v| v as f64);
        partial.age.get_or_insert(age.unwrap_or(55.2));
        partial.transpose()
    }
}

#[derive(Debug, Clone, IntoSqlTable)]
pub enum VideoUrl {
    Direct(String),
    Blob(String),
}

#[derive(Default, Debug, Clone, IntoSqlTable)]
pub enum Availability {
    Now {
        player_url: String,
        video_url: VideoUrl,
    },
    #[default]
    Later,
}

#[derive(Default, Debug, Clone, IntoSqlTable)]
pub struct Movie {
    #[silo(primary)]
    title: String,
    url: Vec<String>,
    available: Availability,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Credits {
    cast: Vec<Cast>,
    crew: Vec<Crew>,
}

#[derive(Clone, Debug, Eq, PartialEq, IntoSqlTable)]
pub struct Crew {
    department: String,
    gender: Option<u8>,
    id: u32,
    job: String,
    name: String,
    profile_path: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, IntoSqlTable)]
pub struct Cast {
    id: u32,
    cast_id: u32,
    character: String,
    gender: Option<u8>,
    name: String,
    profile_path: Option<String>,
    order: u8,
}

#[derive(Clone, Debug, PartialEq, IntoSqlTable)]
pub struct Genre {
    #[silo(primary)]
    id: u16,
    name: String,
}

#[derive(Debug, PartialEq, IntoSqlTable, Clone)]
pub struct MovieWithGenres {
    movie_id: u32,
    genre_id: u16,
}

#[derive(Default, Clone, Debug, PartialEq, IntoSqlTable)]
#[silo(migrate)]
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
    // release_date: time::OffsetDateTime,
    runtime: u32,
    homepage: Option<String>,
    #[silo(skip)]
    genres: Vec<Genre>,
    poster_path: Option<String>,
    backdrop_path: Option<String>,
    popularity: f64,
    budget: u64,
    adult: bool,
    #[silo(skip)]
    credits: Option<Credits>,
}

impl MigrationHandler for TmdbMovie {
    fn migrate(
        _string_storage: &mut StaticStringStorage,
        partial: Self::Partial,
        _row: &silo::rusqlite::Row,
        _connection: &silo::rusqlite::Connection,
    ) -> Option<Self> {
        // if partial.release_date.is_none() {
        //     partial.release_date = Some(time::OffsetDateTime::now_utc());
        // }
        partial.transpose()
    }
}

#[derive(Debug, Clone, IntoSqlTable)]
pub struct FutureMovie {
    pub url: String,
}

#[derive(Default, Clone, Debug, IntoSqlTable)]
pub struct MovieWithRatings {
    pub(crate) movie: Movie,
    pub(crate) ratings: TmdbMovie,
}

// const _: () = const { assert!(matches!(Point::COLUMNS.len(), 2)) };
// // const _: () = const { assert!(matches!(Test::COLUMNS.len(), 6)) };
// const _: () = const { assert!(matches!(Fruit::COLUMNS.len(), 1)) };
// const _: () = const { assert!(!matches!(Availability::PARAM_COUNT, 3)) };
// const _: () = const { assert!(matches!(FruitWithData::COLUMNS.len(), 3)) };

#[derive(Debug, IntoSqlTable, Clone)]
struct FooWithVec {
    #[silo(primary)]
    the_id: usize,
    values_todo_keywords: Vec<String>,
    little_list: Vec<u32>,
    non_vec_field: String,
}

#[derive(Debug, IntoSqlTable, Clone)]
struct HasFooWithVecAsChild {
    child: FooWithVec,
    dummy_value: String,
}

fn main() {
    dbg!(Test::COLUMNS);
    let test_db = Database::create_in_memory().unwrap();
    let foo_with_vec = test_db.load::<HasFooWithVecAsChild>().unwrap();
    foo_with_vec
        .insert(HasFooWithVecAsChild {
            child: FooWithVec {
                the_id: 31,
                values_todo_keywords: vec!["hello".into(), "world".into(), "test".into()],
                little_list: vec![42, 43, 44, 45, 46, 47, 48, 49, 421, 422, 423],
                non_vec_field: "Do not duplicate data needlessly".into(),
            },
            dummy_value: "I just think they are neat!".into(),
        })
        .unwrap();
    test_db.save("table-with-vecs.db").unwrap();
    let result = foo_with_vec
        .filter(HasFooWithVecAsChildFilter::default())
        .unwrap();
    dbg!(result);
    let test_db = Database::open("test-before.db").unwrap();
    test_db.check::<Test>().unwrap();
    // test_db.save("test-before.db").unwrap();

    // test.insert(Test {
    //     id: std::time::Instant::now().elapsed().as_nanos() as u32,
    //     value1: Point { x: 12, y: 42 },
    //     value2: "f32::EPSILON".into(),
    //     value3: FruitWithData::Banana {
    //         ripeness: "Very".into(),
    //     },
    //     age: f64::NAN,
    // })
    // .unwrap();

    // assert!(TmdbMovie::default().as_primary_key().is_some());
    // assert!(MovieWithRatings::default().as_primary_key().is_some());

    TestFilter {
        value1: (PointFilter {
            x: silo::SqlColumnFilter::MustBeEqual(12),
            ..Default::default()
        }),
        // value3: (FruitWithDataFilter {
        //     filter: silo::SqlColumnFilter::MustBeEqual("Banana"),
        // }),
        ..Default::default()
    };
    // _ = dbg!(test.filter(f));
    test_db.save("test.db").unwrap();

    // crashtest::crash_test();
    // let table = test_db.load::<TmdbMovie>().unwrap();
    // table
    //     .insert(TmdbMovie {
    //         id: 0,
    //         imdb_id: 0,
    //         title: "Hello!".into(),
    //         tagline: "Hello!".into(),
    //         original_title: "Hello!".into(),
    //         original_language: "Hello!".into(),
    //         overview: None,
    //         runtime: 5,
    //         homepage: None,
    //         poster_path: None,
    //         backdrop_path: None,
    //         popularity: 4.0,
    //         budget: 2,
    //         adult: true,
    //         credits: None,
    //     })
    //     .unwrap();
    // let table = test_db.load::<MovieWithRatings>().unwrap();
    // for e in crashtest::crash_test() {
    //     table.insert(e).unwrap();
    // }
    // let table = test_db.load::<Movie>().unwrap();
    // table
    //     .insert(Movie {
    //         title: "Hello".into(),
    //         url: "dotcom".into(),
    //         available: Availability::Later,
    //     })
    //     .unwrap();
    // dbg!(result.into_iter().map(|f| f.url).collect::<Vec<_>>());

    // let vt = test_db.load2::<FooWithVec>().unwrap();
    // // test_db.check::<FooWithVec>().unwrap();
    // vt.insert(FooWithVec {
    //     iddasda: 0,
    //     values: vec![
    //         "1".into(),
    //         "1".into(),
    //         "1".into(),
    //         "1".into(),
    //         "1".into(),
    //         "1".into(),
    //         "1".into(),
    //         "1".into(),
    //         "1".into(),
    //         "1".into(),
    //         "1".into(),
    //         "1".into(),
    //     ],
    // })
    // .unwrap();
    test_db.save("test.db").unwrap();
    println!("Hello, world!");
}
