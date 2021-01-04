use crate::error::{Error, ErrorKind, Result};
use argon2::{hash_encoded, verify_encoded, Config};
use getrandom::getrandom;
use std::convert::TryInto;

#[derive(Clone)]
pub struct UserTree {
    pub(super) username_password: sled::Tree,
    pub(super) username_standard_gear: sled::Tree,
    pub(super) username_heartrate_rest: sled::Tree,
    pub(super) username_heartrate_max: sled::Tree,
}

impl UserTree {
    pub fn exists(&self, id: &str) -> Result<bool> {
        match self.username_password.contains_key(id)? {
            true => Ok(true),
            false => Err(Error::BadRequest(ErrorKind::NotFound, "User not found")),
        }
    }

    pub fn insert(&self, username: &str, password: &str) -> Result<()> {
        let mut salt = [0u8; 32];
        getrandom(&mut salt).unwrap();

        let hash = hash_encoded(password.as_bytes(), &salt, &Config::default())
            .map_err(|_| Error::BadServerResponse("Password hashing failed"))?;

        self.username_password.insert(username, hash.as_bytes())?;

        Ok(())
    }

    pub fn set_standard_gear(&self, username: &str, gear: &str) -> Result<()> {
        self.username_standard_gear.insert(username, gear)?;

        Ok(())
    }

    pub fn get_standard_gear(&self, username: &str) -> Result<Option<String>> {
        let get = self.username_standard_gear.get(username)?;

        match get {
            Some(x) => Ok(String::from_utf8(x.to_vec()).ok()),
            None => Ok(None),
        }
    }

    pub fn set_heartrate(
        &self,
        username: &str,
        (heartrate_rest, heartrate_max): (u8, u8),
    ) -> Result<()> {
        self.username_heartrate_rest
            .insert(username, &heartrate_rest.to_ne_bytes())?;
        self.username_heartrate_max
            .insert(username, &heartrate_max.to_ne_bytes())?;

        Ok(())
    }

    pub fn get_heartrate(&self, username: &str) -> Result<Option<(u8, u8)>> {
        let heartrate_rest = self.username_heartrate_rest.get(username)?;
        let heartrate_max = self.username_heartrate_max.get(username)?;

        if let (Some(x), Some(y)) = (heartrate_rest, heartrate_max) {
            Ok(Some((
                u8::from_ne_bytes(
                    x.as_ref()
                        .try_into()
                        .map_err(|_| Error::BadServerResponse("Failed to get heart rate"))?,
                ),
                u8::from_ne_bytes(
                    y.as_ref()
                        .try_into()
                        .map_err(|_| Error::BadServerResponse("Failed to get heart rate"))?,
                ),
            )))
        } else {
            Ok(None)
        }
    }

    pub fn verify_hash(&self, id: &str, password: &str) -> Result<bool> {
        let hash = String::from_utf8(
            self.username_password
                .get(&id)?
                .ok_or(Error::BadRequest(
                    ErrorKind::BadRequest,
                    "Password not found in database",
                ))?
                .to_vec(),
        )
        .map_err(|_| Error::BadServerResponse("Password in database is invalid"))?;

        match verify_encoded(&hash, password.as_bytes()) {
            Ok(true) => Ok(true),
            _ => Err(Error::BadRequest(ErrorKind::NotFound, "Incorrect password")),
        }
    }

    pub fn iter_id(&self) -> Result<impl Iterator<Item = String>> {
        Ok(self
            .username_password
            .iter()
            .keys()
            .flatten()
            .flat_map(|x| String::from_utf8(x.to_vec())))
    }
}
