# MVDB: Minimum Viable (Psuedo) Database

Have you ever thought to yourself, "I would like to keep some data persistently, but can't be bothered to do it well or performantly"? Well, you've found just the right library.

If your use case is:

* Very rare writes, but lots of reads
* Data is shared across multiple threads
* Your data structure is not particularly large
* You are already using `Serde` to serialize some or all of your data
* Your use case feels a little too simple to use even `sqlite`
* Your data format/schema never changes, or only changes by adding, or you are willing to handle migrations yourself
* Optional: Your data is hashable

Then you might like `mvdb`!

## Example

```rust
#[macro_use] extern crate serde_derive;
extern crate mvdb;

use std::path::Path;

use mvdb::Mvdb;
use mvdb::errors::*;

#[derive(Deserialize, Serialize, Debug, Default, Hash)]
struct MyData {
    just_one: InnerData,
    multiple: Vec<InnerData>,
}

#[derive(Deserialize, Serialize, Debug, Default, Hash)]
struct InnerData {
    foo: String,
    bar: Vec<u8>,
    baz: String,
}

fn run() -> Result<()> {
    // Create the database and storage file. If `demo.json` does not exist,
    // it will be created with default values
    let file = Path::new("demo.json");
    let db: Mvdb<MyData> = Mvdb::from_file_or_default(&file)?;

    // Access the database contents atomically via a closure. You may
    // optionally return a value (of any type) from the closure, which will
    // be wrapped in a Result. Immutable reads are made from memory only,
    // and will not result in any file access or writes
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
    // be wrapped in a Result. When using the `use-hashable` feature, changes
    // will be written if the database contents changed. Otherwise, the file
    // will be rewritten after every `access_mut()`
    let z = "thisisatest".into();
    let x = InnerData {
        foo: "tacos".into(),
        bar: vec!(0, 1, 2),
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
```

## License

`mvdb` is licensed under the MIT license.