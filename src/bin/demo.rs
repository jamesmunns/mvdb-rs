#[macro_use]
extern crate serde_derive;
extern crate mvdb;

#[cfg(feature = "use-json")]
extern crate serde_json;

#[cfg(feature = "use-toml")]
extern crate toml;

use std::path::Path;

use mvdb::{Mvdb};
use mvdb::errors::*;

#[derive(Deserialize, Serialize, Debug, Default, Hash)]
struct NotADb {
    just_one: InnerData,
    multiple: Vec<InnerData>,
}

#[derive(Deserialize, Serialize, Debug, Default, Hash)]
struct InnerData {
    foo: String,
    bar: Vec<u8>,
    baz: String,
}

fn ugly(s: &str) -> ::std::result::Result<NotADb, serde_json::Error> {
    serde_json::from_str(s)
}

fn run() -> Result<()> {
    // Create the database and storage file. If `demo.json` does not exist,
    // it will be created with default values
    let file = Path::new("demo.json");
    let db: Mvdb<NotADb, _> = Mvdb::from_file(&file,
                                              serde_json::to_string_pretty,
                                              ugly)?;

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
