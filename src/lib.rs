#[macro_use] extern crate error_chain;
extern crate serde;
extern crate serde_json; // TODO: generic across all serializers/deserializers?

use std::fs::File;
use std::io::prelude::*;
use std::ops::Deref;
use std::ops::DerefMut;
use std::sync::{Arc, Mutex, MutexGuard};

use serde::Serialize;
use serde::de::DeserializeOwned;

error_chain!{}

pub struct Mvdb<T>
{
    inner: Arc<Mutex<T>>,
    file_path: String, // TODO pathbuf
}

impl<T> Mvdb<T>
    where T: Serialize + DeserializeOwned
{
    pub fn new(data: T, path: String) -> Result<Self> {
        let new_self = Self {
            inner: Arc::new(Mutex::new(data)),
            file_path: path
        };

        new_self.write()?;

        Ok(new_self)
    }

    pub fn from_file(path: String) -> Result<Self> {
        let contents = just_load(&path)?;
        Self::new(contents, path)
    }

    pub fn write(&self) -> Result<()> {
        if let Ok(inner) = self.inner.lock() {
            self.write_locked(&inner.deref())
        } else {
            bail!("Failed to write")
        }
    }

    fn write_locked(&self, inner: &T) -> Result<()> {
        just_write(&inner.deref(), &self.file_path)
    }

    fn lock(&self) -> Result<MutexGuard<T>> {
        match self.inner.lock() {
            Err(_) => bail!("failed to lock"),
            Ok(lock) => Ok(lock),
        }
    }

    pub fn access_mut<F>(&self, action: F) -> Result<()>
        where F : FnOnce(&mut T)
    {
        let mut x = self.lock()?;
        let mut y = x.deref_mut();
        action(y);
        self.write_locked(y)
    }

    pub fn access<F,R>(&self, action: F) -> Result<R>
        where F : Fn(&T) -> R
    {
        let mut x = self.lock()?;
        let y = x.deref_mut();
        Ok(action(y))
    }
}

impl<T> Mvdb<T>
    where T: Serialize + DeserializeOwned + Default
{
    pub fn from_file_or_default(path: String) -> Result<Self> {
        match just_load(&path) {
            Ok(data) => Self::new(data, path),
            Err(_) => Self::new(T::default(), path)
        }
    }
}

pub fn just_load<T>(path: &str) -> Result<T>
    where T: DeserializeOwned
{
    let mut file = File::open(&path).chain_err(|| format!("Failed to open file: {}", &path))?;
    let mut contents = String::new();
    let _ = file.read_to_string(&mut contents);
    serde_json::from_str(&contents).chain_err(|| "Deserialize error")
}

pub fn just_write<T>(contents: &T, path: &str) -> Result<()>
    where T: Serialize
{
    let mut file = File::create(path).chain_err(|| format!("Failed to create file: {}", path))?;
    let _ = file.write_all(&serde_json::to_string(contents).chain_err(|| "Failed to serialize")?.into_bytes()).chain_err(|| "Failed to write to file")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
    }
}

