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

pub type MvdbSerializer<T, E> = fn(&T)
    -> ::std::result::Result<String, E>;
pub type MvdbDeserializer<T, E> = fn(&str)
    -> ::std::result::Result<T, E>;

/// Minimum Viable Psuedo Database
pub struct Mvdb<T, E> {
    inner: Arc<Mutex<T>>,
    file_path: PathBuf,
    serializer: MvdbSerializer<T, E>,
    deserializer: MvdbDeserializer<T, E>,
}

/// Implement `Clone` manually, otherwise Rust expects `T` to also impl `Clone`,
/// which is not necessary
impl<T, E> Clone for Mvdb<T, E> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            file_path: self.file_path.clone(),
            serializer: self.serializer,
            deserializer: self.deserializer,
        }
    }
}

impl<T, E> Mvdb<T, E>
where
    T: Serialize + DeserializeOwned,
{
    /// Create a new `Mvdb` given data to contain and path to store.
    /// File will be created and written to immediately
    pub fn new(
        data: T,
        path: &Path,
        ser: MvdbSerializer<T, E>,
        deser: MvdbDeserializer<T, E>,
    ) -> Result<Self> {
        let new_self = Self::new_no_write(data, path, ser, deser);
        new_self.write()?;
        Ok(new_self)
    }

    /// Create a new `Self`, but do not flush to file
    fn new_no_write(
        data: T,
        path: &Path,
        ser: MvdbSerializer<T, E>,
        deser: MvdbDeserializer<T, E>,
    ) -> Self {
        Self {
            inner: Arc::new(Mutex::new(data)),
            file_path: path.to_path_buf(),
            serializer: ser,
            deserializer: deser,
        }
    }

    /// Create a new `Mvdb` given just the path. If the file does
    /// not exist, or the contained data does not match the schema
    /// of `T`, this will return an Error
    pub fn from_file(
        path: &Path,
        ser: MvdbSerializer<T, E>,
        deser: MvdbDeserializer<T, E>,
    ) -> Result<Self> {
        let contents = Self::just_load(deser, &path)?;
        Ok(Self::new_no_write(contents, path, ser, deser))
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
        let (_, hash_before) = self.hash_by_serialize(&y)?;
        let ret = action(y);
        let (ser, hash_after) = self.hash_by_serialize(&y)?;

        if hash_before != hash_after {
            Self::just_write_string(&self.file_path, &ser)?;
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
        Self::just_write(self.serializer, &self.file_path, inner)
    }

    /// Return the MutexGuard for `Mvdb`
    fn lock(&self) -> Result<MutexGuard<T>> {
        match self.inner.lock() {
            Err(_) => bail!("failed to lock"),
            Ok(lock) => Ok(lock),
        }
    }

    /// Use the default hasher to obtain the hash of an item
    fn hash_by_serialize(&self, data: &T) -> Result<(String, u64)>
    where
        T: Serialize,
    {
        let mut hasher = DefaultHasher::new();
        let serialized = match (self.serializer)(data) {
            Ok(ser) => ser,
            Err(_) => bail!("failed to serialize"),
        };
        serialized.hash(&mut hasher);
        Ok((serialized, hasher.finish()))
    }

    /// Attempt to write the contents to a serialized file.
    /// Useful when the contents have already been serialized
    pub fn just_write_string(path: &PathBuf, contents: &str) -> Result<()> {
        let mut file = File::create(path)
            .chain_err(|| format!("Failed to create file: {:?}", path))?;
        let _ = file.write_all(contents.as_bytes())
            .chain_err(|| "Failed to write to file")?;
        Ok(())
    }

    /// Attempt to write the contents of a `T` to a serialized file.
    /// If anything goes wrong (file not writable, serialization failed),
    //  an error will be returned
    pub fn just_write(ser: MvdbSerializer<T, E>, path: &PathBuf, data: &T) -> Result<()> {
        let serialized = match (ser)(data) {
            Ok(ser) => ser,
            Err(_) => bail!("failed to serialize"),
        };
        Self::just_write_string(path, &serialized)
    }

    /// Attempt to load the contents of a serialized file to a `T`.
    /// If anything goes wrong (file not available, schema mismatch),
    //  an error will be returned
    pub fn just_load(deser: MvdbDeserializer<T, E>, path: &Path) -> Result<T> {
        let mut file = File::open(path)
            .chain_err(|| format!("Failed to open file: {:?}", &path))?;
        let mut contents = String::new();
        let _ = file.read_to_string(&mut contents);

        match (deser)(&contents) {
            Ok(deser) => Ok(deser),
            Err(_) => bail!("failed to deserialize"),
        }
    }
}



// impl<T,E> Mvdb<T,E>
// where
//     T: Serialize + DeserializeOwned + Default,
// {
//     /// Create a new `Mvdb` given data to contain and path to store.
//     /// File will be created and written to immediately
//     pub fn from_file_or_default(&self, path: &Path, ser: MvdbSerializer<T,E>, deser: MvdbDeserializer<T,E>) -> Result<Self> {
//         match self.just_load(path) {
//             Ok(data) => Ok(Self::new_no_write(data, path, ser, deser)),
//             Err(_) => Self::new(T::default(), path, ser, deser),
//         }
//     }
// }


#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {}
}
