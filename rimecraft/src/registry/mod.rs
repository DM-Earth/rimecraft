pub mod entry;
pub mod registries;
pub mod tag;

use std::{fmt::Display, slice::Iter};

use datafixerupper::serialization::{DynamicOps, Keyable, Lifecycle};

use crate::util::{collection::IndexedIterable, Identifier};

pub struct RegistryKey<T> {
    registry: Identifier,
    value: Identifier,
    _none: Option<T>,
}

impl<T> RegistryKey<T> {
    pub fn new(registry: Identifier, value: Identifier) -> Self {
        Self {
            registry,
            value,
            _none: None,
        }
    }

    pub fn of<V>(registry: &RegistryKey<V>, value: Identifier) -> Self
    where
        V: Registry<T>,
    {
        Self::new(registry.value.clone(), value)
    }

    pub fn of_registry<V>(registry: Identifier) -> Self
    where
        T: Registry<V>,
    {
        Self::new(registries::root_key(), registry)
    }

    pub fn is_of<V, A>(&self, registry: &RegistryKey<V>) -> bool
    where
        V: Registry<A>,
    {
        self.registry.eq(registry.get_value())
    }

    pub fn try_cast<V, A>(&self, registry_ref: &RegistryKey<V>) -> Option<RegistryKey<V>>
    where
        V: Registry<A>,
    {
        if self.is_of(registry_ref) {
            Some(RegistryKey {
                registry: self.registry.clone(),
                value: self.value.clone(),
                _none: None,
            })
        } else {
            None
        }
    }

    pub fn get_value(&self) -> &Identifier {
        &self.value
    }

    pub fn get_registry(&self) -> &Identifier {
        &self.registry
    }
}

impl<T> Display for RegistryKey<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("RegistryKey[")?;
        self.registry.fmt(f)?;
        f.write_str(" / ")?;
        self.value.fmt(f)?;
        f.write_str("]")?;
        std::fmt::Result::Ok(())
    }
}

impl<T> Clone for RegistryKey<T> {
    fn clone(&self) -> Self {
        Self {
            registry: self.registry.clone(),
            value: self.value.clone(),
            _none: None,
        }
    }
}

impl<T> PartialEq for RegistryKey<T> {
    fn eq(&self, other: &Self) -> bool {
        self.registry == other.registry && self.value == other.value
    }
}

pub trait Registry<T>: Keyable + IndexedIterable<T> {
    fn keys<V>(&self, ops: &impl DynamicOps<V>) -> Iter<V> {
        todo!()
    }

    fn get_self_key(&self) -> &RegistryKey<Self>;

    fn get_id<'a>(&'a self, obj: &'a T) -> Option<&'a Identifier>;
    fn get_key<'a>(&'a self, obj: &'a T) -> Option<&'a RegistryKey<T>>;

    fn get_from_key<'a>(&'a self, key: &RegistryKey<T>) -> Option<&'a T>;
    fn get_from_id<'a>(&'a self, id: &Identifier) -> Option<&'a T>;

    fn get_entry_lifecycle<'a>(&'a self, entry: &'a T) -> &Lifecycle;
    fn get_lifecycle(&self) -> &Lifecycle;

    fn get_ids(&self) -> Vec<&Identifier>;
    fn get_entry_set(&self) -> Vec<(&RegistryKey<T>, &T)>;
    fn get_keys(&self) -> Vec<&RegistryKey<T>>;

    fn contains_id(&self, id: &Identifier) -> bool;
    fn contains(&self, key: &RegistryKey<T>) -> bool;

    fn freeze(&mut self);
}