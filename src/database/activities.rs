use crate::{Activity, Duration, Lap, Record, Session, UserTotals};
use anyhow::{anyhow, Result};
use chrono::{self, Datelike, Local};

#[derive(Clone)]
pub struct ActivityTree {
    pub usernameid_id: sled::Tree,
    pub usernameid_username: sled::Tree,
    pub(super) usernameid_gear: sled::Tree,
    pub(super) usernamegearid_id: sled::Tree,
    pub(super) usernameid_session: sled::Tree,
    pub(super) usernameid_record: sled::Tree,
    pub(super) usernameid_lap: sled::Tree,
}

impl ActivityTree {
    pub fn exists(&self, username: &str, id: &str) -> Result<bool> {
        let mut key = username.as_bytes().to_vec();
        key.push(0xff);
        key.extend_from_slice(id.as_bytes());
        Ok(self.usernameid_id.contains_key(&key)?)
    }

    pub fn insert(&self, activity: Activity, username: &str) -> Result<()> {
        if !self.exists(username, &activity.id)? {
            self.insert_or_overwrite(activity, username)
        } else {
            Ok(())
        }
    }

    pub fn insert_or_overwrite(&self, activity: Activity, username: &str) -> Result<()> {
        let mut key = username.as_bytes().to_vec();
        key.push(0xff);
        key.extend_from_slice(&activity.id.as_bytes());

        let session = bincode::serialize(&activity.session)?;
        self.usernameid_session.insert(&key, session)?;

        let record = bincode::serialize(&activity.record)?;
        self.usernameid_record.insert(&key, record)?;

        let lap = bincode::serialize(&activity.lap)?;
        self.usernameid_lap.insert(&key, lap)?;

        self.usernameid_id.insert(&key, activity.id.as_bytes())?;

        if let Some(x) = activity.gear_id {
            self.usernameid_gear.insert(&key, x.as_bytes())?;

            let mut key = username.as_bytes().to_vec();
            key.push(0xff);
            key.extend_from_slice(&x.as_bytes());
            key.push(0xff);
            key.extend_from_slice(&activity.id.as_bytes());

            self.usernamegearid_id
                .insert(&key, activity.id.as_bytes())?;
        }

        self.usernameid_username.insert(&key, username.as_bytes())?;

        Ok(())
    }

    pub fn user_totals(&self, username: &str) -> Result<UserTotals> {
        let iter = self
            .username_iter_session(username)?
            .collect::<Vec<Session>>();
        let cycling_iter = iter.iter().filter(|x| x.activity_type.is_cycling());

        let running_iter = iter.iter().filter(|x| x.activity_type.is_running());

        let cycling_month = cycling_iter
            .clone()
            .filter(|x| x.start_time.0 > (Local::now() - chrono::Duration::days(30)))
            .fold((0.0, Duration::new(), 0), |acc, x| {
                (
                    acc.0 + x.distance.unwrap_or(0.0),
                    acc.1 + x.duration_active,
                    acc.2 + 1,
                )
            });

        let running_month = running_iter
            .clone()
            .filter(|x| x.start_time.0 > (Local::now() - chrono::Duration::days(30)))
            .fold((0.0, Duration::new(), 0), |acc, x| {
                (
                    acc.0 + x.distance.unwrap_or(0.0),
                    acc.1 + x.duration_active,
                    acc.2 + 1,
                )
            });

        let cycling_year = cycling_iter
            .clone()
            .filter(|x| x.start_time.0.year() == Local::now().year())
            .fold((0.0, Duration::new(), 0), |acc, x| {
                (
                    acc.0 + x.distance.unwrap_or(0.0),
                    acc.1 + x.duration_active,
                    acc.2 + 1,
                )
            });

        let running_year = running_iter
            .clone()
            .filter(|x| x.start_time.0.year() == Local::now().year())
            .fold((0.0, Duration::new(), 0), |acc, x| {
                (
                    acc.0 + x.distance.unwrap_or(0.0),
                    acc.1 + x.duration_active,
                    acc.2 + 1,
                )
            });

        let cycling_all = cycling_iter.fold((0.0, Duration::new(), 0), |acc, x| {
            (
                acc.0 + x.distance.unwrap_or(0.0),
                acc.1 + x.duration_active,
                acc.2 + 1,
            )
        });

        let running_all = running_iter.fold((0.0, Duration::new(), 0), |acc, x| {
            (
                acc.0 + x.distance.unwrap_or(0.0),
                acc.1 + x.duration_active,
                acc.2 + 1,
            )
        });

        Ok(UserTotals {
            cycling_month,
            cycling_year,
            cycling_all,
            running_month,
            running_year,
            running_all,
        })
    }

    pub fn gear_totals(&self, username: &str, gear: &str) -> (f64, Duration) {
        self.username_gear_iter_id(username, gear)
            .unwrap()
            .flat_map(|x| self.get_session(username, &x))
            .fold((0.0, Duration::new()), |acc, x| {
                (acc.0 + x.distance.unwrap_or(0.0), acc.1 + x.duration_active)
            })
    }

    pub fn username_gear_iter_id(
        &self,
        username: &str,
        gear: &str,
    ) -> Result<impl Iterator<Item = String>> {
        let mut prefix = username.as_bytes().to_vec();
        prefix.push(0xff);
        prefix.extend_from_slice(gear.as_bytes());

        Ok(self
            .usernamegearid_id
            .scan_prefix(&prefix)
            .values()
            .rev()
            .flatten()
            .flat_map(|x| String::from_utf8(x.to_vec())))
    }

    pub fn username_iter_session(&self, username: &str) -> Result<impl Iterator<Item = Session>> {
        let mut prefix = username.as_bytes().to_vec();
        prefix.push(0xff);

        Ok(self
            .usernameid_session
            .scan_prefix(&prefix)
            .values()
            .rev()
            .flatten()
            .flat_map(|x| bincode::deserialize::<Session>(&x)))
    }

    pub fn username_iter_id(&self, username: &str) -> Result<impl Iterator<Item = String>> {
        let mut prefix = username.as_bytes().to_vec();
        prefix.push(0xff);

        Ok(self
            .usernameid_id
            .scan_prefix(&prefix)
            .values()
            .rev()
            .flatten()
            .flat_map(|x| String::from_utf8(x.to_vec())))
    }

    pub fn iter_username(&self) -> Result<impl Iterator<Item = String>> {
        Ok(self
            .usernameid_username
            .iter()
            .values()
            .rev()
            .flatten()
            .flat_map(|x| String::from_utf8(x.to_vec())))
    }

    pub fn iter_session(&self) -> Result<impl Iterator<Item = Session>> {
        Ok(self
            .usernameid_session
            .iter()
            .values()
            .rev()
            .flatten()
            .flat_map(|x| bincode::deserialize::<Session>(&x)))
    }

    pub fn iter_record(&self) -> Result<impl Iterator<Item = Record>> {
        Ok(self
            .usernameid_record
            .iter()
            .values()
            .rev()
            .flatten()
            .flat_map(|x| bincode::deserialize::<Record>(&x)))
    }

    pub fn iter_id(&self) -> Result<impl Iterator<Item = String>> {
        Ok(self
            .usernameid_id
            .iter()
            .values()
            .rev()
            .flatten()
            .flat_map(|x| String::from_utf8(x.to_vec())))
    }

    pub fn get_session(&self, username: &str, id: &str) -> Result<Session> {
        let mut key = username.as_bytes().to_vec();
        key.push(0xff);
        key.extend_from_slice(id.as_bytes());

        let get = self.usernameid_session.get(&key)?;

        match get {
            Some(x) => Ok(bincode::deserialize::<Session>(&x)?),
            None => Err(anyhow!("Session not found")),
        }
    }

    pub fn get_record(&self, username: &str, id: &str) -> Result<Record> {
        let mut key = username.as_bytes().to_vec();
        key.push(0xff);
        key.extend_from_slice(id.as_bytes());

        let get = self.usernameid_record.get(&key)?;

        match get {
            Some(x) => Ok(bincode::deserialize::<Record>(&x)?),
            None => Err(anyhow!("Record not found")),
        }
    }

    pub fn get_lap(&self, username: &str, id: &str) -> Result<Vec<Lap>> {
        let mut key = username.as_bytes().to_vec();
        key.push(0xff);
        key.extend_from_slice(id.as_bytes());

        let get = self.usernameid_lap.get(&key)?;

        match get {
            Some(x) => Ok(bincode::deserialize::<Vec<Lap>>(&x)?),
            None => Err(anyhow!("Lap not found")),
        }
    }

    pub fn get_gear_id(&self, username: &str, id: &str) -> Result<Option<String>> {
        let mut key = username.as_bytes().to_vec();
        key.push(0xff);
        key.extend_from_slice(id.as_bytes());

        let get = self.usernameid_gear.get(&key)?;

        match get {
            Some(x) => Ok(Some(String::from_utf8(x.to_vec())?)),
            None => Ok(None),
        }
    }

    pub fn get_activity(&self, username: &str, id: &str) -> Result<Activity> {
        Ok(Activity {
            id: id.to_owned(),
            gear_id: self.get_gear_id(username, id)?,
            session: self.get_session(username, id)?,
            record: self.get_record(username, id)?,
            lap: self.get_lap(username, id)?,
        })
    }
}
