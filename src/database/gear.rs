use crate::{Error, Result, Gear};

#[derive(Clone)]
pub struct GearTree {
    pub(super) usernameid_gear: sled::Tree,
}

impl GearTree {
    pub fn exists(&self, username: String, id: String) -> Result<bool> {
        let mut key = username.as_bytes().to_vec();
        key.push(0xff);
        key.extend_from_slice(id.as_bytes());

        Ok(self.usernameid_gear.contains_key(&key)?)
    }

    pub fn insert(&self, gear: Gear, username: String) -> Result<()> {
        let mut key = username.as_bytes().to_vec();
        key.push(0xff);
        key.extend_from_slice(gear.name.as_bytes());

        let serialized = bincode::serialize(&gear).expect("Failed to serialize gear");
        self.usernameid_gear.insert(key, serialized)?;

        Ok(())
    }

    pub fn iter(&self, username: String) -> sled::Iter {
        let mut prefix = username.as_bytes().to_vec();
        prefix.push(0xff);

        self.usernameid_gear.scan_prefix(&prefix)
    }

    pub fn get(&self, username: String, id: String) -> Result<Gear> {
        let mut key = username.as_bytes().to_vec();
        key.push(0xff);
        key.extend_from_slice(id.as_bytes());

        let get = self.usernameid_gear.get(&key)?;
        Ok(bincode::deserialize::<Gear>(&get.unwrap()).unwrap())
    }
}