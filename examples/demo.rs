// MIT License
//
// Copyright (c) 2017 Anthony James Munns
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

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
    let db: Mvdb<NotADb> = Mvdb::from_file_or_default(&file)?;

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
