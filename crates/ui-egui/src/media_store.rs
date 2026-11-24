//! Always-indexed in-memory media store for the editor.
//!
//! Preserves insertion order and first-match lookup semantics while keeping
//! name-to-index lookup and total byte accounting exact on every mutation.

use std::collections::HashMap;
use std::ops::Deref;
use std::sync::Arc;

/// Ordered, always-indexed store of `(name, bytes)` media entries.
///
/// Dereferences read-only to `&[(String, Arc<[u8]>)]`. Mutations must go
/// through methods on `MediaStore` so the lookup index, total byte count,
/// and generation counter can never fall out of sync.
#[derive(Clone, Debug, Default)]
pub struct MediaStore {
    entries: Vec<(String, Arc<[u8]>)>,
    index: HashMap<String, usize>,
    total_bytes: u64,
    generation: u64,
}

impl MediaStore {
    /// Create an empty media store.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Build a media store from an existing vector of `(name, bytes)` entries.
    #[must_use]
    pub fn from_vec(entries: Vec<(String, Arc<[u8]>)>) -> Self {
        let mut store = Self {
            entries,
            index: HashMap::new(),
            total_bytes: 0,
            generation: 0,
        };
        store.rebuild_index_and_total();
        store
    }

    /// View the stored entries as a slice in insertion order.
    #[must_use]
    pub fn as_slice(&self) -> &[(String, Arc<[u8]>)] {
        &self.entries
    }

    /// Look up the first entry matching `name` in O(1) time.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&Arc<[u8]>> {
        let &idx = self.index.get(name)?;
        self.entries.get(idx).map(|(_, bytes)| bytes)
    }

    /// Return `true` if any entry has `name` in O(1) time.
    #[must_use]
    pub fn contains(&self, name: &str) -> bool {
        self.index.contains_key(name)
    }

    /// Total byte length across all stored entries (saturating).
    #[must_use]
    pub fn total_bytes(&self) -> u64 {
        self.total_bytes
    }

    /// Monotonically wrapping counter incremented on every mutation.
    #[must_use]
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Append a new `(name, bytes)` entry, preserving first-seen lookup semantics.
    pub fn push(&mut self, name: String, bytes: Arc<[u8]>) {
        let idx = self.entries.len();
        let added = bytes.len() as u64;
        self.entries.push((name.clone(), bytes));
        self.index.entry(name).or_insert(idx);
        self.total_bytes = self.total_bytes.saturating_add(added);
        self.generation = self.generation.wrapping_add(1);
    }

    /// Replace the bytes of the first entry matching `name`.
    ///
    /// Returns `true` if an entry was updated, or `false` if `name` is absent.
    pub fn replace(&mut self, name: &str, bytes: Arc<[u8]>) -> bool {
        let Some(&idx) = self.index.get(name) else {
            return false;
        };
        let Some((_, slot)) = self.entries.get_mut(idx) else {
            return false;
        };
        let old_len = slot.len() as u64;
        let new_len = bytes.len() as u64;
        *slot = bytes;
        if self.total_bytes == u64::MAX {
            self.total_bytes = self
                .entries
                .iter()
                .fold(0u64, |acc, (_, b)| acc.saturating_add(b.len() as u64));
        } else {
            self.total_bytes = self
                .total_bytes
                .saturating_sub(old_len)
                .saturating_add(new_len);
        }
        self.generation = self.generation.wrapping_add(1);
        true
    }

    /// Replace all entries and rebuild the lookup index and byte total.
    pub fn set_all(&mut self, entries: Vec<(String, Arc<[u8]>)>) {
        self.entries = entries;
        self.rebuild_index_and_total();
        self.generation = self.generation.wrapping_add(1);
    }

    /// Remove all entries.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.index.clear();
        self.total_bytes = 0;
        self.generation = self.generation.wrapping_add(1);
    }

    /// Shorten the store to at most `len` entries, rebuilding the index if shortened.
    pub fn truncate(&mut self, len: usize) {
        if len < self.entries.len() {
            self.entries.truncate(len);
            self.rebuild_index_and_total();
            self.generation = self.generation.wrapping_add(1);
        }
    }

    /// Retain only entries matching predicate `f`, rebuilding the index and byte total.
    pub fn retain(&mut self, f: impl FnMut(&(String, Arc<[u8]>)) -> bool) {
        self.entries.retain(f);
        self.rebuild_index_and_total();
        self.generation = self.generation.wrapping_add(1);
    }

    /// Access the internal first-seen name-to-index map for indexed render/playback lookups.
    pub(crate) fn index(&self) -> &HashMap<String, usize> {
        &self.index
    }

    fn rebuild_index_and_total(&mut self) {
        self.index.clear();
        self.index.reserve(self.entries.len());
        let mut total = 0u64;
        for (idx, (name, bytes)) in self.entries.iter().enumerate() {
            self.index.entry(name.clone()).or_insert(idx);
            total = total.saturating_add(bytes.len() as u64);
        }
        self.total_bytes = total;
    }
}

impl Deref for MediaStore {
    type Target = [(String, Arc<[u8]>)];

    fn deref(&self) -> &Self::Target {
        &self.entries
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bytes(data: &[u8]) -> Arc<[u8]> {
        Arc::from(data)
    }

    fn assert_store_state(store: &MediaStore, len: usize, total_bytes: u64, generation: u64) {
        assert_eq!(store.len(), len);
        assert_eq!(store.total_bytes(), total_bytes);
        assert_eq!(store.generation(), generation);
    }

    fn assert_entry(store: &MediaStore, name: &str, expected: &[u8]) {
        assert!(store.contains(name));
        assert_eq!(store.get(name).map(AsRef::as_ref), Some(expected));
    }

    fn check_push_and_replace(store: &mut MediaStore) {
        store.push("a.png".to_owned(), bytes(b"1234"));
        assert_store_state(store, 1, 4, 1);
        assert_entry(store, "a.png", b"1234");

        store.push("b.png".to_owned(), bytes(b"abcdef"));
        assert_store_state(store, 2, 10, 2);
        assert_eq!(store.as_slice()[0].0, "a.png");
        assert_eq!(store.as_slice()[1].0, "b.png");

        // Duplicate-key first-match semantics on push.
        store.push("a.png".to_owned(), bytes(b"second-a"));
        assert_store_state(store, 3, 18, 3);
        assert_entry(store, "a.png", b"1234");
        assert_eq!(store.index().get("a.png"), Some(&0));

        // Replace hit updates only first entry; miss is a no-op.
        assert!(store.replace("a.png", bytes(b"XY")));
        assert_store_state(store, 3, 16, 4);
        assert_entry(store, "a.png", b"XY");
        assert_eq!(store.as_slice()[2].1.as_ref(), b"second-a");

        assert!(!store.replace("missing.png", bytes(b"zzz")));
        assert_store_state(store, 3, 16, 4);
    }

    fn check_retain_truncate_set_clear(store: &mut MediaStore) {
        // Retain removing first duplicate promotes second duplicate in index.
        let mut saw_first_a = false;
        store.retain(|(name, _)| {
            if name == "a.png" && !saw_first_a {
                saw_first_a = true;
                false
            } else {
                true
            }
        });
        assert_store_state(store, 2, 14, 5);
        assert_eq!(store.as_slice()[0].0, "b.png");
        assert_entry(store, "a.png", b"second-a");
        assert_eq!(store.index().get("a.png"), Some(&1));

        // Truncate no-op leaves generation unchanged; shortening rebuilds index.
        store.truncate(10);
        assert_store_state(store, 2, 14, 5);
        store.truncate(1);
        assert_store_state(store, 1, 6, 6);
        assert!(store.contains("b.png"));
        assert!(!store.contains("a.png"));
        assert!(store.get("a.png").is_none());

        // set_all rebuilds with first-match semantics.
        store.set_all(vec![
            ("dup.png".to_owned(), bytes(b"first")),
            ("other.png".to_owned(), bytes(b"123")),
            ("dup.png".to_owned(), bytes(b"later")),
        ]);
        assert_store_state(store, 3, 13, 7);
        assert_entry(store, "dup.png", b"first");
        assert_entry(store, "other.png", b"123");
        assert!(!store.contains("b.png"));

        // clear empties entries & index, zeroes total_bytes, bumps generation.
        store.clear();
        assert!(store.is_empty());
        assert_store_state(store, 0, 0, 8);
        assert!(!store.contains("dup.png"));
    }

    #[test]
    fn r1_05_media_store_invariants() {
        let mut store = MediaStore::new();
        assert!(store.is_empty());
        assert_store_state(&store, 0, 0, 0);
        assert!(!store.contains("a.png"));
        assert!(store.get("a.png").is_none());

        check_push_and_replace(&mut store);
        check_retain_truncate_set_clear(&mut store);

        // from_vec preserves first-match semantics and byte count.
        let built = MediaStore::from_vec(vec![
            ("x.png".to_owned(), bytes(b"aa")),
            ("x.png".to_owned(), bytes(b"bbbb")),
        ]);
        assert_store_state(&built, 2, 6, 0);
        assert_entry(&built, "x.png", b"aa");
    }
}
