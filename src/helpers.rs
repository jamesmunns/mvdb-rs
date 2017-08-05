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

//! Convenience functions used internally for loading from files, writing to files, and hashing data
//!
//! It is not intended that users will want or need these, however they are exposed in case users
//! find them interesting

use std::io::prelude::*;
use std::hash::{Hash, Hasher};
use std::fs::File;
use std::path::Path;

use serde::Serialize;
use serde::de::DeserializeOwned;

use std::collections::hash_map::DefaultHasher;
use serde_json;
use errors::*;

/// Use the default hasher to obtain the hash of a serialized item
pub fn hash_by_serialize<T>(data: &T, pretty: bool) -> Result<(String, u64)>
where
    T: Serialize,
{
    let serializer = match pretty {
        true => serde_json::to_string_pretty,
        false => serde_json::to_string,
    };

    let mut hasher = DefaultHasher::new();
    let serialized = serializer(data)
        .chain_err(|| "Failed to serialize for hashing")?;
    serialized.hash(&mut hasher);
    Ok((serialized, hasher.finish()))
}

/// Attempt to load the contents of a serialized file to a `T`
///
/// If anything goes wrong (file not available, schema mismatch),
/// an error will be returned
pub fn just_load<T>(path: &Path) -> Result<T>
where
    T: DeserializeOwned,
{
    let mut file = File::open(path)
        .chain_err(|| format!("Failed to open file: {:?}", &path))?;
    let mut contents = String::new();
    let _ = file.read_to_string(&mut contents);
    serde_json::from_str(&contents).chain_err(|| "Deserialize error")
}

/// Attempt to write the contents of a `T` to a serialized file
///
/// If anything goes wrong (file not writable, serialization failed),
/// an error will be returned
pub fn just_write<T>(contents: &T, path: &Path, pretty: bool) -> Result<()>
where
    T: Serialize,
{
    let serializer = match pretty {
        true => serde_json::to_string_pretty,
        false => serde_json::to_string,
    };

    just_write_string(&serializer(contents)
        .chain_err(|| "Failed to serialize")?, path)
}


/// Attempt to write the contents to a serialized file
///
/// Useful when the contents have already been serialized
pub fn just_write_string(contents: &str, path: &Path) -> Result<()>
{
    let mut file = File::create(path)
        .chain_err(|| format!("Failed to create file: {:?}", path))?;
    let _ = file.write_all(contents.as_bytes())
        .chain_err(|| "Failed to write to file")?;
    Ok(())
}