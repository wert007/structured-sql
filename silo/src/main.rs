use std::ops::Add;

#[derive(Default, Debug, Clone, silo::derive::ToTable)]
// #[silo(primary_key)]
struct AddressTT {
    // pk: PrimaryKey,
    #[silo(primary)]
    pk: u64,
    city: String,
    street: String,
}

// #[derive(Default, Debug, Clone, ToColumns)]
// struct AddressC {
//     city: String,
//     street: String,
// }

#[derive(Default, Debug, Clone, silo::derive::ToTable)]
struct Person {
    name: String,
    age: u8,
    // #[silo(foreign)]
    // residence: AddressTT,
    residence: AddressTT,
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
    println!("Hello, world!");
}
