use crate::{
    item::Item,
    nbt::{compound, NbtCompound, NbtElement, NbtTagSizeTracker},
    network::packet::PacketBytes,
    registry::{registries, tag::TagKey, Registry},
    util::{collection::IndexedIterable, Identifier},
};
use bytes::{Buf, BufMut};

pub trait TransferVariant<O>: Sized {
    fn is_blank(&self) -> bool;
    fn get_raw_id(&self) -> usize;
    fn get_nbt(&self) -> Option<&NbtCompound>;
    fn get_nbt_mut(&mut self) -> Option<&mut NbtCompound>;
    fn to_nbt(&self) -> NbtCompound;
    fn to_packet<T: Buf + BufMut>(&self, buf: &mut PacketBytes<T>);

    fn from_nbt(nbt: &NbtCompound) -> Self;
    fn from_packet<T: Buf + BufMut>(buf: &mut PacketBytes<T>) -> Self;

    fn set_nbt(&mut self, nbt: NbtCompound);

    fn has_nbt(&self) -> bool {
        self.get_nbt().is_some()
    }

    fn get_or_create_nbt(&mut self) -> &NbtCompound {
        if !self.has_nbt() {
            self.set_nbt(NbtCompound::new())
        }
        self.get_nbt().unwrap()
    }

    fn get_or_create_nbt_mut(&mut self) -> &mut NbtCompound {
        if !self.has_nbt() {
            self.set_nbt(NbtCompound::new())
        }
        self.get_nbt_mut().unwrap()
    }

    fn nbt_matches(&self, other: &NbtCompound) -> bool {
        self.get_nbt().map_or_else(|| false, |e| e.eq(other))
    }

    fn clone_nbt(&self) -> Option<NbtCompound> {
        self.get_nbt().cloned()
    }

    fn clone_or_create_nbt(&self) -> NbtCompound {
        self.clone_nbt().unwrap_or(NbtCompound::new())
    }

    fn get_from_registry<'a>(&self, registry: &'a Registry<O>) -> Option<&'a O> {
        registry.get_from_raw_id(self.get_raw_id())
    }
}

#[derive(PartialEq, Clone)]
pub struct ItemVariant {
    raw_id: usize,
    nbt: Option<NbtCompound>,
}

impl ItemVariant {
    pub fn new(id: usize, nbt: Option<NbtCompound>) -> Self {
        Self { raw_id: id, nbt }
    }

    pub fn set_nbt(&mut self, nbt: Option<NbtCompound>) {
        self.nbt = nbt
    }

    pub fn is_in(&self, registry: &Registry<Item>, tag_key: &TagKey<Item>) -> bool {
        match registry.get_entry_from_raw_id(self.raw_id) {
            Some(entry) => entry.get_tags().iter().any(|t| t == &tag_key),
            _ => false,
        }
    }
}

impl Default for ItemVariant {
    fn default() -> Self {
        Self {
            raw_id: registries::ITEM.read().unwrap().get_default_raw_id(),
            nbt: None,
        }
    }
}

impl TransferVariant<Item> for ItemVariant {
    fn is_blank(&self) -> bool {
        self.raw_id == registries::ITEM.read().unwrap().get_default_raw_id()
    }

    fn get_raw_id(&self) -> usize {
        self.raw_id
    }

    fn get_nbt(&self) -> Option<&NbtCompound> {
        match &self.nbt {
            Some(e) => Some(e),
            None => None,
        }
    }

    fn get_nbt_mut(&mut self) -> Option<&mut NbtCompound> {
        match &mut self.nbt {
            Some(e) => Some(e),
            None => None,
        }
    }

    fn set_nbt(&mut self, nbt: NbtCompound) {
        self.nbt = Some(nbt);
    }

    fn to_nbt(&self) -> NbtCompound {
        let mut result = NbtCompound::new();
        result.insert(
            "item".to_string(),
            NbtElement::String(
                registries::ITEM
                    .read()
                    .unwrap()
                    .get_entry_from_raw_id(self.raw_id)
                    .unwrap()
                    .get_key()
                    .unwrap()
                    .get_value()
                    .to_string(),
            ),
        );
        if self.nbt.is_some() {
            result.insert(
                "tag".to_string(),
                NbtElement::Compound(self.nbt.clone().unwrap()),
            );
        }
        result
    }

    fn to_packet<T: Buf + BufMut>(&self, buf: &mut PacketBytes<T>) {
        if self.is_blank() {
            buf.put_bool(false);
        } else {
            buf.put_bool(true);
            buf.put_u32(self.raw_id as u32);
            buf.put_nbt(self.nbt.clone()).unwrap();
        }
    }

    fn from_nbt(tag: &NbtCompound) -> Self {
        let registry = registries::ITEM.read().unwrap();
        let item = registry
            .get_raw_id_from_id(
                &match Identifier::parse(compound::get_str(tag, "item").to_string()) {
                    Some(id) => id,
                    None => registry.get_default_id().clone(),
                },
            )
            .unwrap_or(registry.get_default_raw_id());
        let nbt = compound::get_compound(tag, "tag").cloned();
        Self::new(item, nbt)
    }

    fn from_packet<T: Buf + BufMut>(buf: &mut PacketBytes<T>) -> Self {
        if !buf.get_bool() {
            Self::default()
        } else {
            let item = buf.get_u32() as usize;
            let nbt = match buf.get_nbt(&mut NbtTagSizeTracker::default()) {
                Ok(Some(e)) => Some(e),
                _ => None,
            };
            Self::new(item, nbt)
        }
    }
}
