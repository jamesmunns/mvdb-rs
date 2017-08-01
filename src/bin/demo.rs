#[macro_use]
extern crate serde_derive;
extern crate mvdb;

use std::path::Path;

use mvdb::Mvdb;
use mvdb::errors::*;

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

fn run() -> Result<()> {
    // Create the database and storage file. If `demo.json` does not exist,
    // it will be created with default values
    let file = Path::new("demo.json");
    let db: Mvdb<NotADb> = Mvdb::from_file_or_default(&file, false)?;

    // Access the database contents atomically via a closure. You may
    // optionally return a value (of any type) from the closure, which will
    // be wrapped in a Result
    let y = db.access(|data| {
        // Data can be used immutably within the access
        for i in data.multiple.iter() {
            println!("baz: {}", i.baz);
        }

        // When returning data, it must be cloned, as references must not
        // outlive the atomic lock
        data.just_one.foo.clone()
    })?;
    println!("y: {:?}", y);

    // Access the database contents atomically via a closure. You may
    // optionally return a value (of any type) from the closure, which will
    // be wrapped in a Result. Changes will be written if the database contents
    // changed
    let z = "thisisatest".into();
    let x = InnerData {
        foo: "tacos".into(),
        bar: vec![0, 1, 2],
        baz: "burritos".into(),
    };

    db.access_mut(|data: &mut NotADb| {
        data.just_one.foo = z;
        data.multiple.push(x);
    })?;

    Ok(())
}

fn main() {
    assert!(run().is_ok());
}
