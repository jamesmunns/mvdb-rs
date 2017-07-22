extern crate serde;
#[macro_use] extern crate serde_derive;
extern crate serde_json;

extern crate mvdb;

use mvdb::Mvdb;

#[derive(Deserialize, Serialize, Debug, Default)]
struct NotADb {
    just_one: InnerData,
    multiple: Vec<InnerData>,
}

#[derive(Deserialize, Serialize, Debug, Default)]
struct InnerData {
    foo: String,
    bar: Vec<u8>,
    baz: String,
}

fn main() {
    let db: Mvdb<NotADb> = Mvdb::from_file_or_default("demo.json".to_string()).unwrap();

    let y = db.access(|data| data.just_one.foo.clone()).unwrap();
    println!("y: {:?}", y);

    let z = "thisisatest".to_string();
    db.access_mut(|data: &mut NotADb| data.just_one.foo = z).unwrap();
}