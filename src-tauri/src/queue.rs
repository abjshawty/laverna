use crate::ytdlp::args::ItemOptions;
use crate::ytdlp::progress::Progress;
use serde::{Deserialize, Serialize};
use std::path::Path;

pub type ItemId = u64;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Metadata {
    pub title: Option<String>,
    pub thumbnail: Option<String>,
    pub duration: Option<f64>,
    /// The metadata lookup ran (successfully or not). Downloads wait for it,
    /// because a playlist URL must be expanded first.
    #[serde(default)]
    pub resolved: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Status {
    Pending,
    Downloading {
        #[serde(skip)]
        progress: Option<Progress>,
    },
    Done { path: Option<String> },
    /// Skipped because it is already in the download archive.
    Skipped,
    Failed { error: String, hint_update: bool },
    Cancelled,
}

impl Status {
    pub fn is_finished(&self) -> bool {
        matches!(self, Status::Done { .. } | Status::Skipped)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Item {
    pub id: ItemId,
    pub url: String,
    pub options: ItemOptions,
    #[serde(default)]
    pub meta: Metadata,
    pub status: Status,
    /// Raw yt-dlp output of the last attempt; not persisted.
    #[serde(skip)]
    pub log: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueueError {
    NotFound(ItemId),
    /// The operation isn't allowed in the item's current status.
    InvalidState(ItemId),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Queue {
    items: Vec<Item>,
    next_id: ItemId,
    /// The worker only picks up items while not paused.
    pub paused: bool,
}

impl Queue {
    #[cfg(test)]
    pub fn items(&self) -> &[Item] {
        &self.items
    }

    pub fn get(&self, id: ItemId) -> Option<&Item> {
        self.items.iter().find(|i| i.id == id)
    }

    fn get_mut(&mut self, id: ItemId) -> Result<&mut Item, QueueError> {
        self.items.iter_mut().find(|i| i.id == id).ok_or(QueueError::NotFound(id))
    }

    pub fn add(&mut self, url: String, options: ItemOptions) -> ItemId {
        let id = self.next_id;
        self.next_id += 1;
        self.items.push(Item {
            id,
            url,
            options,
            meta: Metadata::default(),
            status: Status::Pending,
            log: Vec::new(),
        });
        id
    }

    /// Replaces a playlist entry with its videos, in place, each inheriting its options.
    pub fn expand(&mut self, id: ItemId, entries: Vec<(String, Metadata)>) -> Result<Vec<ItemId>, QueueError> {
        let pos = self.items.iter().position(|i| i.id == id).ok_or(QueueError::NotFound(id))?;
        if self.items[pos].status != Status::Pending {
            return Err(QueueError::InvalidState(id));
        }
        let options = self.items[pos].options.clone();
        let new: Vec<Item> = entries
            .into_iter()
            .map(|(url, meta)| {
                let id = self.next_id;
                self.next_id += 1;
                Item { id, url, options: options.clone(), meta, status: Status::Pending, log: Vec::new() }
            })
            .collect();
        let ids = new.iter().map(|i| i.id).collect();
        self.items.splice(pos..=pos, new);
        Ok(ids)
    }

    pub fn unresolved(&self) -> Vec<ItemId> {
        self.items.iter().filter(|i| !i.meta.resolved).map(|i| i.id).collect()
    }

    pub fn log(&self, id: ItemId) -> Option<&[String]> {
        self.get(id).map(|i| i.log.as_slice())
    }

    pub fn set_meta(&mut self, id: ItemId, meta: Metadata) -> Result<(), QueueError> {
        self.get_mut(id)?.meta = meta;
        Ok(())
    }

    /// Only pending items are editable.
    pub fn set_options(&mut self, id: ItemId, options: ItemOptions) -> Result<(), QueueError> {
        let item = self.get_mut(id)?;
        if item.status != Status::Pending {
            return Err(QueueError::InvalidState(id));
        }
        item.options = options;
        Ok(())
    }

    /// Removing the downloading item is not allowed; cancel it first.
    pub fn remove(&mut self, id: ItemId) -> Result<Item, QueueError> {
        let pos = self.items.iter().position(|i| i.id == id).ok_or(QueueError::NotFound(id))?;
        if matches!(self.items[pos].status, Status::Downloading { .. }) {
            return Err(QueueError::InvalidState(id));
        }
        Ok(self.items.remove(pos))
    }

    /// Moves a pending item to `index` in the full list.
    pub fn move_to(&mut self, id: ItemId, index: usize) -> Result<(), QueueError> {
        let pos = self.items.iter().position(|i| i.id == id).ok_or(QueueError::NotFound(id))?;
        if self.items[pos].status != Status::Pending {
            return Err(QueueError::InvalidState(id));
        }
        let item = self.items.remove(pos);
        let index = index.min(self.items.len());
        self.items.insert(index, item);
        Ok(())
    }

    pub fn current(&self) -> Option<&Item> {
        self.items.iter().find(|i| matches!(i.status, Status::Downloading { .. }))
    }

    pub fn can_start(&self) -> bool {
        !self.paused
            && self.current().is_none()
            && self.items.iter().any(|i| i.status == Status::Pending && i.meta.resolved)
    }

    /// Picks the next pending item and marks it downloading. `None` if paused, busy, or empty.
    pub fn start_next(&mut self) -> Option<&Item> {
        if self.paused || self.current().is_some() {
            return None;
        }
        let item = self
            .items
            .iter_mut()
            .find(|i| i.status == Status::Pending && i.meta.resolved)?;
        item.status = Status::Downloading { progress: None };
        item.log.clear();
        Some(item)
    }

    fn downloading_mut(&mut self, id: ItemId) -> Result<&mut Item, QueueError> {
        let item = self.get_mut(id)?;
        if !matches!(item.status, Status::Downloading { .. }) {
            return Err(QueueError::InvalidState(id));
        }
        Ok(item)
    }

    pub fn report_progress(&mut self, id: ItemId, progress: Progress) -> Result<(), QueueError> {
        self.downloading_mut(id)?.status = Status::Downloading { progress: Some(progress) };
        Ok(())
    }

    pub fn push_log(&mut self, id: ItemId, line: String) -> Result<(), QueueError> {
        self.get_mut(id)?.log.push(line);
        Ok(())
    }

    /// yt-dlp exited 0. No final path means the archive skipped it.
    pub fn finish(&mut self, id: ItemId, path: Option<String>) -> Result<(), QueueError> {
        let item = self.downloading_mut(id)?;
        item.status = match path {
            Some(p) => Status::Done { path: Some(p) },
            None => Status::Skipped,
        };
        item.options.force = false;
        Ok(())
    }

    pub fn fail(&mut self, id: ItemId, error: String, hint_update: bool) -> Result<(), QueueError> {
        self.downloading_mut(id)?.status = Status::Failed { error, hint_update };
        Ok(())
    }

    pub fn cancel(&mut self, id: ItemId) -> Result<(), QueueError> {
        self.downloading_mut(id)?.status = Status::Cancelled;
        Ok(())
    }

    /// Failed/cancelled → pending. Skipped → pending with `force` ("download anyway").
    pub fn retry(&mut self, id: ItemId) -> Result<(), QueueError> {
        let item = self.get_mut(id)?;
        match item.status {
            Status::Failed { .. } | Status::Cancelled => {}
            Status::Skipped => item.options.force = true,
            _ => return Err(QueueError::InvalidState(id)),
        }
        item.status = Status::Pending;
        Ok(())
    }

    /// Pausing stops the current download; it goes back to pending so resume picks it up first.
    /// Returns the id that the caller must kill.
    pub fn pause(&mut self) -> Option<ItemId> {
        self.paused = true;
        let item = self.items.iter_mut().find(|i| matches!(i.status, Status::Downloading { .. }))?;
        item.status = Status::Pending;
        Some(item.id)
    }

    pub fn resume(&mut self) {
        self.paused = false;
    }

    pub fn clear_finished(&mut self) {
        self.items.retain(|i| !i.status.is_finished());
    }

    pub fn remaining(&self) -> usize {
        self.items
            .iter()
            .filter(|i| matches!(i.status, Status::Pending | Status::Downloading { .. }))
            .count()
    }

    /// Loads a saved queue. An interrupted download becomes pending and the queue starts paused.
    pub fn load(path: &Path) -> Self {
        let mut q: Queue = std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        for item in &mut q.items {
            if matches!(item.status, Status::Downloading { .. }) {
                item.status = Status::Pending;
            }
        }
        if q.remaining() > 0 {
            q.paused = true;
        }
        q
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_string_pretty(self)?)?;
        std::fs::rename(tmp, path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Preset;

    fn opts() -> ItemOptions {
        ItemOptions { preset: Preset::Best, subtitles: false, force: false }
    }

    fn queue_of(n: usize) -> (Queue, Vec<ItemId>) {
        let mut q = Queue::default();
        let ids: Vec<_> = (0..n).map(|i| q.add(format!("u{i}"), opts())).collect();
        for id in &ids {
            q.set_meta(*id, resolved()).unwrap();
        }
        (q, ids)
    }

    fn resolved() -> Metadata {
        Metadata { resolved: true, ..Default::default() }
    }

    #[test]
    fn unresolved_items_wait_for_metadata() {
        let mut q = Queue::default();
        let a = q.add("playlist".into(), opts());
        assert!(q.start_next().is_none());
        assert_eq!(q.unresolved(), vec![a]);
        q.set_meta(a, resolved()).unwrap();
        assert_eq!(q.start_next().unwrap().id, a);
    }

    #[test]
    fn runs_strictly_one_at_a_time_in_order() {
        let (mut q, ids) = queue_of(3);
        assert_eq!(q.start_next().unwrap().id, ids[0]);
        assert!(q.start_next().is_none(), "second item must wait");
        q.finish(ids[0], Some("/a".into())).unwrap();
        assert_eq!(q.start_next().unwrap().id, ids[1]);
    }

    #[test]
    fn failure_moves_on() {
        let (mut q, ids) = queue_of(2);
        q.start_next();
        q.fail(ids[0], "boom".into(), false).unwrap();
        assert_eq!(q.start_next().unwrap().id, ids[1]);
    }

    #[test]
    fn finish_without_path_is_skipped_and_retry_forces() {
        let (mut q, ids) = queue_of(1);
        q.start_next();
        q.finish(ids[0], None).unwrap();
        assert_eq!(q.get(ids[0]).unwrap().status, Status::Skipped);
        q.retry(ids[0]).unwrap();
        let item = q.get(ids[0]).unwrap();
        assert_eq!(item.status, Status::Pending);
        assert!(item.options.force);
    }

    #[test]
    fn cancel_then_retry() {
        let (mut q, ids) = queue_of(1);
        q.start_next();
        q.cancel(ids[0]).unwrap();
        assert_eq!(q.retry(ids[0]), Ok(()));
        assert_eq!(q.retry(ids[0]), Err(QueueError::InvalidState(ids[0])));
    }

    #[test]
    fn pause_returns_current_to_front_and_blocks() {
        let (mut q, ids) = queue_of(2);
        q.start_next();
        assert_eq!(q.pause(), Some(ids[0]));
        assert!(q.start_next().is_none());
        q.resume();
        assert_eq!(q.start_next().unwrap().id, ids[0]);
    }

    #[test]
    fn only_pending_items_are_editable_or_movable() {
        let (mut q, ids) = queue_of(3);
        q.start_next();
        assert_eq!(q.set_options(ids[0], opts()), Err(QueueError::InvalidState(ids[0])));
        assert_eq!(q.move_to(ids[0], 2), Err(QueueError::InvalidState(ids[0])));
        assert!(q.remove(ids[0]).is_err());
        q.move_to(ids[2], 1).unwrap();
        let order: Vec<_> = q.items().iter().map(|i| i.id).collect();
        assert_eq!(order, vec![ids[0], ids[2], ids[1]]);
    }

    #[test]
    fn expand_replaces_playlist_in_place() {
        let (mut q, ids) = queue_of(2);
        let new = q
            .expand(ids[0], vec![("a".into(), resolved()), ("b".into(), resolved())])
            .unwrap();
        let urls: Vec<_> = q.items().iter().map(|i| i.url.as_str()).collect();
        assert_eq!(urls, vec!["a", "b", "u1"]);
        assert_eq!(new.len(), 2);
        assert!(q.get(ids[0]).is_none());
    }

    #[test]
    fn clear_finished_keeps_failed() {
        let (mut q, ids) = queue_of(3);
        q.start_next();
        q.finish(ids[0], Some("/a".into())).unwrap();
        q.start_next();
        q.fail(ids[1], "x".into(), false).unwrap();
        q.clear_finished();
        let left: Vec<_> = q.items().iter().map(|i| i.id).collect();
        assert_eq!(left, vec![ids[1], ids[2]]);
    }

    #[test]
    fn reload_turns_interrupted_into_paused_pending() {
        let dir = std::env::temp_dir().join(format!("laverna-test-{}", std::process::id()));
        let path = dir.join("queue.json");
        let (mut q, ids) = queue_of(2);
        q.start_next();
        q.save(&path).unwrap();
        let loaded = Queue::load(&path);
        assert!(loaded.paused);
        assert_eq!(loaded.get(ids[0]).unwrap().status, Status::Pending);
        let mut loaded = loaded;
        assert_eq!(loaded.add("new".into(), opts()), 2, "ids keep increasing after reload");
        std::fs::remove_dir_all(dir).ok();
    }
}
