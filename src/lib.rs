#[macro_use]
extern crate error_chain;
extern crate serde;

// TODO: generic across all serializers/deserializers?
#[cfg(feature = "use-json")]
extern crate serde_json;
#[cfg(feature = "use-toml")]
extern crate toml;

use std::fs::File;
use std::io::prelude::*;
use std::ops::Deref;
use std::ops::DerefMut;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

use serde::Serialize;
use serde::de::DeserializeOwned;

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

pub mod errors {
    error_chain!{}
}

use errors::*;

/// Minimum Viable Psuedo Database
pub struct Mvdb<T> {
    inner: Arc<Mutex<T>>,
    file_path: PathBuf,
}

/// Implement `Clone` manually, otherwise Rust expects `T` to also impl `Clone`,
/// which is not necessary
impl<T> Clone for Mvdb<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            file_path: self.file_path.clone(),
        }
    }
}

impl<T> Mvdb<T>
where
    T: Serialize + DeserializeOwned,
{
    /// Create a new `Mvdb` given data to contain and path to store.
    /// File will be created and written to immediately
    pub fn new(data: T, path: &Path) -> Result<Self> {
        let new_self = Self::new_no_write(data, path);
        new_self.write()?;
        Ok(new_self)
    }

    /// Create a new `Self`, but do not flush to file
    fn new_no_write(data: T, path: &Path) -> Self {
        Self {
            inner: Arc::new(Mutex::new(data)),
            file_path: path.to_path_buf(),
        }
    }

    /// Create a new `Mvdb` given just the path. If the file does
    /// not exist, or the contained data does not match the schema
    /// of `T`, this will return an Error
    pub fn from_file(path: &Path) -> Result<Self> {
        let contents = just_load(&path)?;
        Ok(Self::new_no_write(contents, path))
    }

    /// Provide atomic writable access to the database contents via a closure.
    /// If the hash of the contents after the access has changed, the database
    /// will be written to the file.
    pub fn access_mut<F, R>(&self, action: F) -> Result<R>
    where
        F: FnOnce(&mut T) -> R,
    {
        let mut x = self.lock()?;
        let mut y = x.deref_mut();
        let (_, hash_before) = hash_by_serialize(&y)?;
        let ret = action(y);
        let (ser, hash_after) = hash_by_serialize(&y)?;

        if hash_before != hash_after {
            just_write_string(&ser, &self.file_path)?;
        }

        Ok(ret)
    }

    /// Provide atomic read-only access to the database contents via a closure.
    /// Contents are accessed in-memory only, and will not re-read from the
    /// storage file, or cause any writes
    pub fn access<F, R>(&self, action: F) -> Result<R>
    where
        F: Fn(&T) -> R,
    {
        let x = self.lock()?;
        let y = x.deref();
        Ok(action(y))
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
        just_write(&inner.deref(), &self.file_path)
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
    /// Create a new `Mvdb` given data to contain and path to store.
    /// File will be created and written to immediately
    pub fn from_file_or_default(path: &Path) -> Result<Self> {
        match just_load(path) {
            Ok(data) => Ok(Self::new_no_write(data, path)),
            Err(_) => Self::new(T::default(), path),
        }
    }
}

/// Use the default hasher to obtain the hash of an item
#[cfg(feature = "use-json")]
fn hash_by_serialize<T>(data: &T) -> Result<(String, u64)>
where
    T: Serialize,
{
    let mut hasher = DefaultHasher::new();
    let serialized = serde_json::to_string(data)
        .chain_err(|| "Failed to serialize for hashing")?;
    serialized.hash(&mut hasher);
    Ok((serialized, hasher.finish()))
}

/// Use the default hasher to obtain the hash of an item
#[cfg(feature = "use-toml")]
fn hash_by_serialize<T>(data: &T) -> Result<(String, u64)>
where
    T: Serialize,
{
    let mut hasher = DefaultHasher::new();
    let serialized = toml::to_string(data)
        .chain_err(|| "Failed to serialize for hashing")?;
    serialized.hash(&mut hasher);
    Ok((serialized, hasher.finish()))
}

/// Attempt to load the contents of a serialized file to a `T`.
/// If anything goes wrong (file not available, schema mismatch),
//  an error will be returned
#[cfg(feature = "use-json")]
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

/// Attempt to load the contents of a serialized file to a `T`.
/// If anything goes wrong (file not available, schema mismatch),
//  an error will be returned
#[cfg(feature = "use-toml")]
pub fn just_load<T>(path: &Path) -> Result<T>
where
    T: DeserializeOwned,
{
    let mut file = File::open(path)
        .chain_err(|| format!("Failed to open file: {:?}", &path))?;
    let mut contents = String::new();
    let _ = file.read_to_string(&mut contents);
    toml::from_str(&contents).chain_err(|| "Deserialize error")
}

/// Attempt to write the contents of a `T` to a serialized file.
/// If anything goes wrong (file not writable, serialization failed),
//  an error will be returned
#[cfg(feature = "use-json")]
pub fn just_write<T>(contents: &T, path: &Path) -> Result<()>
where
    T: Serialize,
{
    just_write_string(&serde_json::to_string(contents)
        .chain_err(|| "Failed to serialize")?, path)
}

/// Attempt to write the contents of a `T` to a serialized file.
/// If anything goes wrong (file not writable, serialization failed),
//  an error will be returned
#[cfg(feature = "use-toml")]
pub fn just_write<T>(contents: &T, path: &Path) -> Result<()>
where
    T: Serialize,
{
    just_write_string(&toml::to_string(contents)
        .chain_err(|| "Failed to serialize")?, path)
}

/// Attempt to write the contents to a serialized file.
/// Useful when the contents have already been serialized
pub fn just_write_string(contents: &str, path: &Path) -> Result<()>
{
    let mut file = File::create(path)
        .chain_err(|| format!("Failed to create file: {:?}", path))?;
    let _ = file.write_all(contents.as_bytes())
        .chain_err(|| "Failed to write to file")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {}
}
