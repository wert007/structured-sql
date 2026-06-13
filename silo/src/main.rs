#[derive(Default, Debug, Clone, silo::derive::ToTable)]
// #[silo(primary_key)]
struct AddressTT {
    // pk: PrimaryKey,
    #[silo(primary)]
    pk: u64,
    city: String,
    street: String,
}

#[derive(Default, Debug, Clone, ToColumns)]
struct AddressTC {
    city: String,
    street: String,
}

// #[derive(Debug, silo::derive::ToColumns)]
// enum MovieRole {
//     FilmStar,
//     Director,
//     Watcher,
// }

#[derive(Default, Debug, Clone, silo::derive::ToTable)]
struct Person {
    name: String,
    age: u8,
    // #[silo(foreign)]
    // residence: AddressTT,
    residence: AddressTC,
    // role: MovieRole,
}

fn main() {
    use silo::{Database, SqlTable};

    let db = Database::create_in_memory().unwrap();
    let persons = db.load::<Person>().unwrap();
    persons
        .insert(Person {
            name: "Johnny English".into(),
            age: 58,
            residence: AddressTT {
                pk: 0,
                city: "Toronot".into(),
                street: "Bakerstreet 221b".into(),
            },
        })
        .unwrap();
    db.save("file.sqlite").unwrap();

    // persons.load_all();
    let r = persons
        .load_where(|f| {
            f.and(
                f.name_equals("Johnny English"),
                f.or(f.age_less_than(60), f.age_greater_than(70)),
            )
        })
        .unwrap();
    dbg!(r);
    // persons.load_where(|f| {
    //     f.name_equals("Johnny english")
    //         .and()
    //         .age_less_than(60)
    //         .or()
    //         .age_greater_than(70)
    // });
    println!("Hello, world!");
}
