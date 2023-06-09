mod registries;
pub mod tag;

use std::ops::Deref;

use crate::prelude::*;

pub use registries::*;

/// Represents a registration and its id and tags.
pub struct Holder<T> {
    key: RegistryKey<T>,
    pub tags: parking_lot::RwLock<Vec<tag::TagKey<T>>>,
    value: T,
}

impl<T> Holder<T> {
    pub fn key(&self) -> &RegistryKey<T> {
        &self.key
    }

    /// If this registration is in target tag.
    pub fn is_in(&self, tag: &tag::TagKey<T>) -> bool {
        self.tags.read().contains(tag)
    }
}

impl<T> Deref for Holder<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

/// Immutable registry storing entries with mutable tag bindings.
///
/// You're not able to create a registry directly, use a [`Builder`] instead.
pub struct Registry<T> {
    default: Option<usize>,
    entries: Vec<Holder<T>>,
    id_map: hashbrown::HashMap<Identifier, usize>,
    /// Key of this registry.
    pub key: RegistryKey<Self>,
    key_map: hashbrown::HashMap<RegistryKey<T>, usize>,
    /// Tag to entries mapping of this registry.
    pub tags: parking_lot::RwLock<hashbrown::HashMap<tag::TagKey<T>, Vec<usize>>>,
}

impl<T> Registry<T> {
    /// Whether this registry contains an entry with the target registry key.
    pub fn contains_ket(&self, key: &RegistryKey<T>) -> bool {
        self.key_map.contains_key(key)
    }

    /// Whether this registry contains an entry with the target id.
    pub fn contains_id(&self, id: &Identifier) -> bool {
        self.id_map.contains_key(id)
    }

    /// Returns the default entry of this reigstry.
    ///
    /// # Panics
    ///
    /// Panic if a default entry don't exist.
    /// See [`Self::is_defaulted`].
    pub fn default_entry(&self) -> (usize, &Holder<T>) {
        let def = self
            .default
            .expect("trying to get a default entry that don't exist");
        (def, self.get_from_raw(def).unwrap())
    }

    /// Get an entry from a [`RegistryKey`].
    pub fn get_from_key(&self, key: &RegistryKey<T>) -> Option<(usize, &Holder<T>)> {
        self.key_map
            .get(key)
            .map(|e| (*e, self.entries.get(*e).unwrap()))
    }

    /// Get an entry from an [`Identifier`].
    pub fn get_from_id(&self, id: &Identifier) -> Option<(usize, &Holder<T>)> {
        self.id_map
            .get(id)
            .map(|e| (*e, self.entries.get(*e).unwrap()))
    }

    /// Get an entry from its raw id.
    pub fn get_from_raw(&self, raw_id: usize) -> Option<&Holder<T>> {
        self.entries.get(raw_id)
    }

    /// Whether a default entry exist in this registry.
    pub fn is_defaulted(&self) -> bool {
        self.default.is_some()
    }

    /// Returns an iterator over the slice of entries.
    pub fn iter(&self) -> std::slice::Iter<'_, Holder<T>> {
        self.entries.iter()
    }
}

impl<T> std::ops::Index<usize> for Registry<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        &self.get_from_raw(index).unwrap().value
    }
}

impl<T: PartialEq + Eq> crate::util::collections::Indexed<T> for Registry<T> {
    fn get_raw_id(&self, value: &T) -> Option<usize> {
        self.entries
            .iter()
            .enumerate()
            .find(|e| &e.1.value == value)
            .map(|e| e.0)
    }

    fn get(&self, index: usize) -> Option<&T> {
        self.get_from_raw(index).map(|e| &e.value)
    }

    fn len(&self) -> usize {
        self.entries.len()
    }
}

impl<T: PartialEq + Eq> crate::util::collections::Indexed<Holder<T>> for Registry<T> {
    fn get_raw_id(&self, value: &Holder<T>) -> Option<usize> {
        self.entries
            .iter()
            .enumerate()
            .find(|e| e.1 as *const Holder<T> as usize == value as *const Holder<T> as usize)
            .map(|e| e.0)
    }

    fn get(&self, index: usize) -> Option<&Holder<T>> {
        self.get_from_raw(index)
    }

    fn len(&self) -> usize {
        self.entries.len()
    }
}

/// Mutable registry builder for building [`Registry`].
pub struct Builder<T: Registration> {
    entries: Vec<(T, Identifier)>,
}

impl<T: Registration> Builder<T> {
    pub const fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Register a new value and its id into this builder and return its raw id.
    pub fn register(&mut self, value: T, id: Identifier) -> anyhow::Result<usize> {
        if self.entries.iter().any(|e| e.1 == id) {
            Err(anyhow::anyhow!("Registration with id {id} already exist!"))
        } else {
            self.entries.push((value, id));
            Ok(self.entries.len() - 1)
        }
    }
}

impl<T: Registration> crate::util::Freeze<Registry<T>> for Builder<T> {
    type Opts = (RegistryKey<Registry<T>>, Option<Identifier>);

    fn build(self, opts: Self::Opts) -> Registry<T> {
        let entries = self
            .entries
            .into_iter()
            .enumerate()
            .map(|mut e| {
                e.1 .0.accept(e.0);
                Holder {
                    value: e.1 .0,
                    key: RegistryKey::new(&opts.0, e.1 .1.clone()),
                    tags: parking_lot::RwLock::new(Vec::new()),
                }
            })
            .collect::<Vec<_>>();

        let id_map = {
            let mut map = hashbrown::HashMap::new();
            for e in entries.iter().enumerate() {
                map.insert(e.1.key.value().clone(), e.0);
            }
            map
        };

        Registry {
            default: opts.1.map(|e| id_map.get(&e).copied()).flatten(),
            key_map: {
                let mut map = hashbrown::HashMap::new();
                for e in entries.iter().enumerate() {
                    map.insert(e.1.key.clone(), e.0);
                }
                map
            },
            entries,
            id_map,
            key: opts.0,
            tags: parking_lot::RwLock::new(hashbrown::HashMap::new()),
        }
    }
}

/// Registration for storing raw_id.
pub trait Registration {
    /// Accept a raw id.
    fn accept(&mut self, id: usize);
    /// Return the raw id.
    fn raw_id(&self) -> usize;
}

pub trait RegistryAccess: Sized {
    fn registry() -> &'static Registry<Self>;
}

/// Represents a key for a value in a registry in a context where
/// a root registry is available.
///
/// This type is driven by [`std::sync::Arc`] so it's cheap to clone.
pub struct RegistryKey<T> {
    _type: std::marker::PhantomData<T>,
    /// (reg, value)
    inner: std::sync::Arc<(Identifier, Identifier)>,
}

impl<T> RegistryKey<T> {
    pub fn new(registry: &RegistryKey<Registry<T>>, value: Identifier) -> Self {
        Self {
            inner: std::sync::Arc::new((registry.inner.1.clone(), value)),
            _type: std::marker::PhantomData,
        }
    }

    /// Whether this registry key belongs to the given registry.
    pub fn is_of<E>(&self, reg: &RegistryKey<Registry<E>>) -> bool {
        self.inner.0 == reg.inner.1
    }

    /// Return `Some(_)` if the key is of reg, otherwise `None`.
    pub fn cast<E>(&self, reg: &RegistryKey<Registry<E>>) -> Option<RegistryKey<E>> {
        if self.is_of(&reg) {
            Some(RegistryKey {
                inner: std::sync::Arc::clone(&self.inner),
                _type: std::marker::PhantomData,
            })
        } else {
            None
        }
    }

    /// Value of this key.
    pub fn value(&self) -> &Identifier {
        &self.inner.0
    }

    /// Registry of this key.
    pub fn reg(&self) -> &Identifier {
        &self.inner.1
    }
}

impl<T> RegistryKey<Registry<T>> {
    /// Creates a registry key for a registry in the root registry
    /// with an identifier for the registry.
    pub fn of_reg(reg: Identifier) -> Self {
        Self {
            inner: std::sync::Arc::new((registries::root_key(), reg)),
            _type: std::marker::PhantomData,
        }
    }
}

impl<T> std::fmt::Display for RegistryKey<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("RegistryKey[")?;
        self.inner.0.fmt(f)?;
        f.write_str(" / ")?;
        self.inner.1.fmt(f)?;
        f.write_str("]")
    }
}

impl<T> Clone for RegistryKey<T> {
    fn clone(&self) -> Self {
        Self {
            inner: std::sync::Arc::clone(&self.inner),
            _type: std::marker::PhantomData,
        }
    }
}

impl<T> PartialEq for RegistryKey<T> {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl<T> Eq for RegistryKey<T> {}

impl<T> std::hash::Hash for RegistryKey<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.inner.hash(state)
    }
}

/// Freezeable registry for building and freezing registries,
/// just like what MCJE's `Registry` do.
pub type Freezer<T> = crate::util::Freezer<Registry<T>, Builder<T>>;
