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

//! # MVDB: Minimum Viable (Psuedo) Database
//!
//! Have you ever thought to yourself, "I would like to keep some data persistently, but can't be bothered to do it well or performantly"? Well, you've found just the right library.
//!
//! If your use case is:
//!
//! * Very rare writes, but lots of reads
//! * Data access is shared across multiple threads
//! * Your data structure is not particularly large
//! * You are already using `Serde` to serialize some or all of your data
//! * Your use case feels a little too simple to use even `sqlite`
//! * Your data format/schema never changes, or only changes by adding, or you are willing to handle migrations yourself
//!
//! Then you might like `mvdb`!
//!
//! ## How it works
//!
//! `mvdb` takes a `Serializable` and `Deserializable` Rust data structure, and uses `serde_json` to represent this data in
//! a file. After the initial file load, all read-accesses are made from in-memory, rather than re-reading from file. After any
//! mutable or read-write-access, the contents of the data is checked for changes. If the contents have been modified, they
//! will be pushed back to the file. All accesses, read-only and read-write, are made atomically.
//!
//! Access to the structure is made in a transactional manner, via closures. Care should be taken not to block within these closures,
//! as it will block access to the data for all other consumers until the closure completes.
//!
//! ## Put it in your project
//!
//! ```
//! # in Cargo.toml:
//! [dependencies]
//! mvdb = "0.2"
//!
//! # in your Rust code:
//! extern crate mvdb;
//! ```
//!
//! ## Example
//!
//! ```rust
//! [macro_use] extern crate serde_derive;
//! extern crate serde;
//! extern crate mvdb;
//!
//! use std::path::Path;
//! use mvdb::Mvdb;
//!
//! #[derive(Deserialize, Serialize)]
//! struct DemoData {
//!     foo: String,
//!     bar: Vec<u8>,
//!     baz: String,
//! }
//!
//! fn main() {
//!     let file = Path::new("demo.json");
//!     let my_data: Mvdb<DemoData> = Mvdb::from_file(&file)
//!         .expect("File does not exist, or schema mismatch");
//!
//!     // Read access
//!     let foo_from_disk = my_data.access(|db| db.foo.clone())
//!         .expect("Failed to access file");
//!
//!     // Write access
//!     my_data.access_mut(|db: &mut DemoData| {
//!         db.baz = "New Value".into();
//!     }).expect("Failed to access file");
//! }
//! ```
//!
//! ## Warnings
//!
//! ### File Writes and Performance
//!
//! Generally, `mvdb` is not meant to be used as a high performance database, but rather for data that changes rarely, such as updating
//! a token once a day, occasionally adding information, or configuration that can be changed on-the-fly. **Every time data within the
//! structure is changed, the ENTIRE FILE will be rewritten**.
//!
//! If you have fields that change rapidly, but do not need to be persisted to disk, such as a `VecDeque` of messages, you can use
//! the serde `#[skip]` directive to omit this field from storage, and writes to these fields will not cause a write to the
//! backing file. `mvdb` also respects other [Serde Attributes](https://serde.rs/attributes.html), which may be used to affect
//! behavior as desired.
//!
//! ### Schemas
//!
//! `mvdb` makes no attempt to handle schemas, and will fail to load any file that does not match the currently known schema.
//! It is possible to work around this with the mechanisms that Serde provides, please see this [ticket](https://github.com/serde-rs/serde/issues/745),
//! and the linked Reddit thread.
//!
//! ## But I want to use (bincode|toml|something), not JSON!
//!
//! I hope to someday support those too! Check out [this tracking issue](https://github.com/jamesmunns/mvdb-rs/issues/2) for
//! details on blockers and progress on that.
//!
//! ## Pretty Printing
//!
//! All methods for creating a new `mvdb` offer a `_pretty` variant. This will store the contents using pretty-printed JSON,
//! at the cost of additional size. This can be useful during development, or when humans are expected to modify or inspect
//! the stored contents
//!
//! ## Default
//!
//! If the data you are storing implements the `Default` trait, either through `#[derive(Default)]`, or by manually implementing
//! the trait, then you can use the `from_file_or_default` method. This will attempt to load the file, or if that fails, a new file
//! will be created with default data. This is useful for configuration files with sane defaults, or when the file is expected to
//! be generated on first run

#[macro_use]
extern crate error_chain;
extern crate serde;
// TODO: generic across all serializers/deserializers?
extern crate serde_json;

pub mod helpers;
pub mod errors;

mod mvdb;
pub use mvdb::*;
