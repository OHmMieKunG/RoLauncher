use crate::{model::ProcessIdentity, platform};
use serde::{Deserialize, Serialize};
use std::{
    io::Write,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct Baseline {
    pub priority: Option<u32>,
    pub power: Option<[u32; 3]>,
    pub memory_priority: Option<u32>,
    pub affinity: Option<usize>,
    pub window: Option<WindowBaseline>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct WindowBaseline {
    pub handle: u64,
    pub flags: u32,
    pub show: u32,
    pub minimum: [i32; 2],
    pub maximum: [i32; 2],
    pub rectangle: [i32; 4],
    pub visible: bool,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    schema: u32,
    account_id: String,
    identity: ProcessIdentity,
    session: u32,
    baseline: Baseline,
}

pub(crate) struct Journal {
    path: PathBuf,
    account_id: String,
    identity: ProcessIdentity,
    session: u32,
}
impl Journal {
    pub fn new(
        directory: &Path,
        account_id: &str,
        identity: &ProcessIdentity,
        session: u32,
    ) -> Result<Self, String> {
        if account_id.parse::<u64>().is_err()
            || identity.pid == 0
            || identity.creation_time.parse::<u64>().is_err()
            || identity.tracker.is_empty()
        {
            return Err("Invalid restoration identity".into());
        }
        Ok(Self {
            path: directory
                .join("control-journal")
                .join(format!("{account_id}-{}.json", identity.generation)),
            account_id: account_id.into(),
            identity: identity.clone(),
            session,
        })
    }
    pub fn load(&self) -> Result<Option<Baseline>, String> {
        let metadata = match std::fs::metadata(&self.path) {
            Ok(value) => value,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(_) => return Err("Restoration journal unreadable; baseline unavailable".into()),
        };
        if metadata.len() > 16 * 1024 {
            return Err("Restoration journal too large".into());
        }
        let entry: Entry = serde_json::from_slice(
            &std::fs::read(&self.path).map_err(|_| "Restoration journal unreadable")?,
        )
        .map_err(|_| "Restoration journal invalid; baseline unavailable")?;
        if entry.schema != 1
            || entry.account_id != self.account_id
            || entry.identity != self.identity
            || entry.session != self.session
        {
            return Err("Restoration journal ownership mismatch; baseline unavailable".into());
        }
        validate(&entry.baseline)?;
        Ok(Some(entry.baseline))
    }
    pub fn save(&self, baseline: Baseline) -> Result<(), String> {
        validate(&baseline)?;
        std::fs::create_dir_all(self.path.parent().unwrap())
            .map_err(|_| "Unable to create restoration journal")?;
        let bytes = serde_json::to_vec(&Entry {
            schema: 1,
            account_id: self.account_id.clone(),
            identity: self.identity.clone(),
            session: self.session,
            baseline,
        })
        .map_err(|_| "Unable to serialize restoration journal")?;
        let temp = self.path.with_extension("json.tmp");
        let mut file =
            std::fs::File::create(&temp).map_err(|_| "Unable to write restoration journal")?;
        file.write_all(&bytes)
            .and_then(|_| file.sync_all())
            .map_err(|_| "Unable to flush restoration journal")?;
        drop(file);
        platform::replace_file(&temp, &self.path)
    }
    pub fn remove(&self) -> Result<(), String> {
        match std::fs::remove_file(&self.path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(_) => Err("Unable to remove restored journal".into()),
        }
    }
}
fn validate(b: &Baseline) -> Result<(), String> {
    if b.priority
        .is_some_and(|p| ![0x40, 0x4000, 0x20, 0x8000, 0x80, 0x100].contains(&p))
        || b.memory_priority.is_some_and(|p| !(1..=5).contains(&p))
        || b.affinity == Some(0)
        || b.power
            .is_some_and(|p| p[0] != 1 || p[1] & !5 != 0 || p[2] & !p[1] != 0)
        || b.window.as_ref().is_some_and(|w| {
            w.handle == 0
                || w.show > 11
                || w.flags & !7 != 0
                || w.rectangle[2] < w.rectangle[0]
                || w.rectangle[3] < w.rectangle[1]
        })
    {
        return Err("Restoration baseline invalid; no settings changed".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn durable_baseline_rejects_corruption_and_stale_ownership() {
        let directory =
            std::env::temp_dir().join(format!("rolauncher-journal-{}", uuid::Uuid::new_v4()));
        let identity = ProcessIdentity {
            pid: 42,
            creation_time: "123456".into(),
            tracker: "fixture".into(),
            generation: uuid::Uuid::new_v4(),
        };
        let journal = Journal::new(&directory, "1", &identity, 2).unwrap();
        assert_eq!(journal.load().unwrap(), None);
        let baseline = Baseline {
            priority: Some(0x20),
            power: Some([1, 1, 1]),
            ..Default::default()
        };
        journal.save(baseline.clone()).unwrap();
        assert_eq!(journal.load().unwrap(), Some(baseline));
        let moved = Journal::new(
            &directory,
            "1",
            &ProcessIdentity {
                generation: uuid::Uuid::new_v4(),
                ..identity.clone()
            },
            2,
        )
        .unwrap();
        std::fs::copy(&journal.path, &moved.path).unwrap();
        assert!(moved.load().is_err());
        moved.remove().unwrap();
        let other_account = Journal::new(&directory, "2", &identity, 2).unwrap();
        std::fs::copy(&journal.path, &other_account.path).unwrap();
        assert!(other_account.load().is_err());
        other_account.remove().unwrap();
        for (pid, created, tracker, session) in [
            (43, "123456", "fixture", 2),
            (42, "123457", "fixture", 2),
            (42, "123456", "other", 2),
            (42, "123456", "fixture", 3),
        ] {
            let changed = ProcessIdentity {
                pid,
                creation_time: created.into(),
                tracker: tracker.into(),
                ..identity.clone()
            };
            assert!(
                Journal::new(&directory, "1", &changed, session)
                    .unwrap()
                    .load()
                    .is_err()
            );
        }
        assert!(
            journal
                .save(Baseline {
                    affinity: Some(0),
                    ..Default::default()
                })
                .is_err()
        );
        std::fs::write(&journal.path, b"{corrupt").unwrap();
        assert!(journal.load().is_err());
        journal.remove().unwrap();
        assert!(journal.load().unwrap().is_none());
        std::fs::remove_dir_all(directory).unwrap();
    }
}
