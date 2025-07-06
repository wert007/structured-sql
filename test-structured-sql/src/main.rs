use silo::{AsParams, Database, IntoSqlTable, IntoSqlVecTable, SqlTable, SqlVecTable};

mod crashtest;

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

#[derive(Debug, IntoSqlTable)]
struct Test {
    #[silo(primary)]
    id: u32,
    value1: Point,
    #[silo(skip)]
    value2: String,
    value3: FruitWithData,
}

#[derive(Debug, Clone, IntoSqlTable)]
pub enum VideoUrl {
    Direct(String),
    Blob(String),
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
pub struct Movie {
    title: String,
    url: String,
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

#[derive(Debug, PartialEq, IntoSqlTable)]
pub struct MovieWithGenres {
    movie_id: u32,
    genre_id: u16,
}

#[derive(Clone, Debug, PartialEq, IntoSqlTable)]
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
    release_date: String,
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

#[derive(Debug, Clone, IntoSqlTable)]
pub struct FutureMovie {
    pub url: String,
}

#[derive(Clone, Debug, IntoSqlTable)]
pub struct MovieWithRatings {
    pub(crate) movie: Movie,
    pub(crate) ratings: TmdbMovie,
}

const _: () = const { assert!(matches!(Point::COLUMNS.len(), 2)) };
// const _: () = const { assert!(matches!(Test::COLUMNS.len(), 6)) };
const _: () = const { assert!(matches!(Fruit::COLUMNS.len(), 1)) };
const _: () = const { assert!(!matches!(Availability::PARAM_COUNT, 3)) };
const _: () = const { assert!(matches!(FruitWithData::COLUMNS.len(), 3)) };

#[derive(Debug, IntoSqlVecTable)]
struct FooWithVec {
    #[silo(primary)]
    iddasda: usize,
    values: Vec<String>,
}

fn main() {
    dbg!(Test::COLUMNS);
    let test_db = Database::create_in_memory().unwrap();
    let test = test_db.load::<Test>().unwrap();
    test_db.save("test-before.db").unwrap();

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
            x: silo::SqlColumnFilter::MustBeEqual(12),
            ..Default::default()
        }),
        // value3: (FruitWithDataFilter {
        //     filter: silo::SqlColumnFilter::MustBeEqual("Banana"),
        // }),
        ..Default::default()
    };
    _ = dbg!(test.filter(f));
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

    let vt = test_db.load2::<FooWithVec>().unwrap();
    vt.insert(FooWithVec {
        iddasda: 0,
        values: vec![
            "1".into(),
            "1".into(),
            "1".into(),
            "1".into(),
            "1".into(),
            "1".into(),
            "1".into(),
            "1".into(),
            "1".into(),
            "1".into(),
            "1".into(),
            "1".into(),
        ],
    })
    .unwrap();
    test_db.save("test.db").unwrap();
    println!("Hello, world!");
}
