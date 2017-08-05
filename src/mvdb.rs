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

use std::ops::Deref;
use std::ops::DerefMut;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

use serde::Serialize;
use serde::de::DeserializeOwned;

use errors::*;
use helpers::*;

/// Minimum Viable Psuedo Database
pub struct Mvdb<T> {
    inner: Arc<Mutex<T>>,
    file_path: PathBuf,
    pretty: bool,
}

/// Implement `Clone` manually, otherwise Rust expects `T` to also impl `Clone`,
/// which is not necessary
impl<T> Clone for Mvdb<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            file_path: self.file_path.clone(),
            pretty: self.pretty,
        }
    }
}

impl<T> Mvdb<T>
where
    T: Serialize + DeserializeOwned,
{
    /// Create a new `Mvdb` given data to contain and path to store.
    /// File will be created and written to immediately
    ///
    /// # Examples
    ///
    /// ```rust
    /// let data = DemoData::new();
    /// let file = Path::new("demo.json");
    ///
    /// let my_data = Mvdb::new(data, &file)
    ///     .expect("Could not write to file");
    /// ```
    pub fn new(data: T, path: &Path) -> Result<Self> {
        Self::new_inner(data, path, false)
    }

    /// Create a new `Mvdb` given data to contain and path to store.
    /// File will be created and written to immediately. Information
    /// will be stored in a "pretty-print" JSON format, at the cost
    /// of additonal storage space and write time
    ///
    /// # Examples
    ///
    /// ```rust
    /// let data = DemoData::new();
    /// let file = Path::new("demo_pretty.json");
    ///
    /// let my_data = Mvdb::new_pretty(data, &file)
    ///     .expect("Could not write to file");
    /// ```
    pub fn new_pretty(data: T, path: &Path) -> Result<Self> {
        Self::new_inner(data, path, true)
    }

    /// Create a new `Mvdb` given just the path. If the file does
    /// not exist, or the contained data does not match the schema
    /// of `T`, this will return an Error
    ///
    /// # Examples
    ///
    /// ```rust
    /// let file = Path::new("demo.json");
    /// let my_data: Mvdb<DemoData> = Mvdb::from_file(&file);
    ///     .expect("File does not exist, or schema mismatch");
    /// ```
    pub fn from_file(path: &Path) -> Result<Self> {
        Self::from_file_inner(path, false)
    }

    /// Create a new `Mvdb` given just the path. If the file does
    /// not exist, or the contained data does not match the schema
    /// of `T`, this will return an Error. Subsequent writes will
    /// be stored in a "pretty-print" JSON format, at the cost of
    /// additional storage space and write time. The file does not
    /// need to be "pretty-printed" before using this function
    ///
    /// # Examples
    ///
    /// ```rust
    /// let file = Path::new("demo_pretty.json");
    /// let my_data: Mvdb<DemoData> = Mvdb::from_file_pretty(&file);
    ///     .expect("File does not exist, or schema mismatch");
    /// ```
    pub fn from_file_pretty(path: &Path) -> Result<Self> {
        Self::from_file_inner(path, true)
    }

    /// Create a new `Mvdb` given data to contain and path to store.
    /// File will be created and written to immediately
    fn new_inner(data: T, path: &Path, pretty: bool) -> Result<Self> {
        let new_self = Self::new_no_write(data, path, pretty);
        new_self.write()?;
        Ok(new_self)
    }

    /// Create a new `Mvdb` given just the path. If the file does
    /// not exist, or the contained data does not match the schema
    /// of `T`, this will return an Error
    fn from_file_inner(path: &Path, pretty: bool) -> Result<Self> {
        let contents = just_load(&path)?;
        Ok(Self::new_no_write(contents, path, pretty))
    }

    /// Create a new `Self`, but do not flush to file
    fn new_no_write(data: T, path: &Path, pretty: bool) -> Self {
        Self {
            inner: Arc::new(Mutex::new(data)),
            file_path: path.to_path_buf(),
            pretty: pretty,
        }
    }

    /// Provide atomic read-only access to the database contents via a closure.
    /// Contents are accessed in-memory only, and will not re-read from the
    /// storage file, or cause any writes
    ///
    /// # Examples
    ///
    /// ```rust
    /// let foo_from_disk = my_data.access(|db| db.foo.clone())
    ///     .expect("Failed to access file");
    /// ```
    pub fn access<F, R>(&self, action: F) -> Result<R>
    where
        F: Fn(&T) -> R,
    {
        let x = self.lock()?;
        let y = x.deref();
        Ok(action(y))
    }

    /// Provide atomic writable access to the database contents via a closure.
    /// If the hash of the serialized contents after the access has changed, the database
    /// will be written to the file.
    ///
    /// # Examples
    ///
    /// ```rust
    /// my_data.access_mut(|db: &mut DemoData| {
    ///     db.baz = "New Value".into();
    /// }).expect("Failed to access file");
    /// ```
    pub fn access_mut<F, R>(&self, action: F) -> Result<R>
    where
        F: FnOnce(&mut T) -> R,
    {
        let mut x = self.lock()?;
        let mut y = x.deref_mut();
        let (_, hash_before) = hash_by_serialize(&y, self.pretty)?;
        let ret = action(y);
        let (ser, hash_after) = hash_by_serialize(&y, self.pretty)?;

        if hash_before != hash_after {
            just_write_string(&ser, &self.file_path)?;
        }

        Ok(ret)
    }

    /// Attempt to write `Self` to file
    fn write(&self) -> Result<()> {
        if let Ok(inner) = self.inner.lock() {
            self.write_locked(&inner.deref())
        } else {
            bail!("Failed to write")
        }
    }

    /// Raw write to file without locks
    fn write_locked(&self, inner: &T) -> Result<()> {
        just_write(&inner.deref(), &self.file_path, self.pretty)
    }

    /// Return the MutexGuard for `Mvdb`
    fn lock(&self) -> Result<MutexGuard<T>> {
        match self.inner.lock() {
            Err(_) => bail!("failed to lock"),
            Ok(lock) => Ok(lock),
        }
    }
}

impl<T> Mvdb<T>
where
    T: Serialize + DeserializeOwned + Default,
{
    /// Attempt to load from a file. If the file does not exist,
    /// or if the schema does not match, a new file will be written
    /// with the default contents of `T`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let file = Path::new("demo.json");
    /// let my_data: Mvdb<DemoData> = Mvdb::from_file_or_default(&file);
    ///     .expect("Could not write to file");
    /// ```
    pub fn from_file_or_default(path: &Path) -> Result<Self> {
        Self::from_file_or_default_inner(path, false)
    }

    /// Attempt to load from a file. If the file does not exist,
    /// or if the schema does not match, a new file will be written
    /// with the default contents of `T`. Any writes made will use
    /// pretty-printed JSON
    ///
    /// # Examples
    ///
    /// ```rust
    /// let file = Path::new("demo_pretty.json");
    /// let my_data: Mvdb<DemoData> = Mvdb::from_file_or_default_pretty(&file);
    ///     .expect("Could not write to file");
    /// ```
    pub fn from_file_or_default_pretty(path: &Path) -> Result<Self> {
        Self::from_file_or_default_inner(path, true)
    }

    /// Attempt to load from a file. If the file does not exist,
    /// or if the schema does not match, a new file will be written
    /// with the default contents of `T`.
    fn from_file_or_default_inner(path: &Path, pretty: bool) -> Result<Self> {
        match just_load(path) {
            Ok(data) => Ok(Self::new_no_write(data, path, pretty)),
            Err(_) => Self::new_inner(T::default(), path, pretty),
        }
    }
}