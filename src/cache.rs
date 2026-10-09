//! Optional process-local derived parsing cache. Entries cannot import authority,
//! raw envelopes, or caller-supplied parse results.
use crate::{
    model::{ParseResult, SourceStatus, Terminal},
    source::SourceSnapshot,
};
use std::collections::{BTreeSet, VecDeque};
const MAX_ENTRIES: usize = 8;
const MAX_BYTES: usize = 8 * 1024 * 1024;
const CACHE_VERSION: &str = "specguard.parse-cache/v1";
const PARSER_VERSION: &str = "specguard.native-explicit/v1";
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CacheStats {
    pub hits: usize,
    pub misses: usize,
    pub bypasses: usize,
}
struct Entry {
    work: String,
    cache_version: &'static str,
    parser_version: &'static str,
    parsed: ParseResult,
    cost: usize,
}
impl Entry {
    fn matches(&self, work: &str) -> bool {
        self.work == work
            && self.cache_version == CACHE_VERSION
            && self.parser_version == PARSER_VERSION
    }
}
pub(crate) struct Pending(Entry);
/// A disabled-by-default, bounded FIFO cache, owned by one local controller.
/// It deliberately has no Serialize/Deserialize or public insert operation.
/// ```compile_fail
/// let cache: specguard::cache::ParseCache = serde_json::from_str("{}").unwrap();
/// ```
pub struct ParseCache {
    enabled: bool,
    max_entries: usize,
    max_bytes: usize,
    charged: usize,
    entries: VecDeque<Entry>,
    stats: CacheStats,
}
impl Default for ParseCache {
    fn default() -> Self {
        Self::disabled()
    }
}
impl ParseCache {
    pub fn enabled() -> Self {
        Self {
            enabled: true,
            ..Self::disabled()
        }
    }
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            max_entries: MAX_ENTRIES,
            max_bytes: MAX_BYTES,
            charged: 0,
            entries: VecDeque::new(),
            stats: CacheStats::default(),
        }
    }
    /// Controllers may lower the fixed ceilings; higher or zero limits reject.
    pub fn with_limits(entries: usize, bytes: usize) -> Result<Self, &'static str> {
        if entries == 0 || entries > MAX_ENTRIES || bytes == 0 || bytes > MAX_BYTES {
            return Err("cache capacity budget");
        }
        Ok(Self {
            max_entries: entries,
            max_bytes: bytes,
            ..Self::enabled()
        })
    }
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    /// Conservative retained-entry charge, not measured RSS.
    pub fn charged_bytes(&self) -> usize {
        self.charged
    }
    pub fn stats(&self) -> CacheStats {
        self.stats
    }
    pub(crate) fn get(
        &mut self,
        work: &str,
        snapshot: &SourceSnapshot,
        control: &mut impl FnMut(Option<&SourceStatus>) -> Result<(), ()>,
    ) -> Result<Option<ParseResult>, ()> {
        if !self.enabled {
            self.stats.bypasses = self.stats.bypasses.saturating_add(1);
            return Ok(None);
        }
        let entry = self.entries.iter().find(|e| e.matches(work));
        let Some(entry) = entry else {
            self.stats.misses = self.stats.misses.saturating_add(1);
            return Ok(None);
        };
        if entry.cost > self.max_bytes
            || !cacheable(snapshot, &entry.parsed)
            || charge(&entry.parsed, self.max_bytes) != Some(entry.cost)
        {
            self.stats.misses = self.stats.misses.saturating_add(1);
            return Ok(None);
        }
        // Same callback order as parse_controlled, including real cancellation
        // before every source. Unique one-to-one paths are required below.
        control(None)?;
        for source in &snapshot.inventory.entries {
            control(None)?;
            let status = entry
                .parsed
                .sources
                .iter()
                .find(|s| s.path == source.path)
                .expect("cacheable exact source set");
            control(Some(status))?;
        }
        self.stats.hits = self.stats.hits.saturating_add(1);
        Ok(Some(entry.parsed.clone()))
    }
    pub(crate) fn stage(
        &mut self,
        work: &str,
        snapshot: &SourceSnapshot,
        parsed: &ParseResult,
    ) -> Option<Pending> {
        if !self.enabled {
            return None;
        }
        if work.len() > 256 || !cacheable(snapshot, parsed) {
            self.stats.bypasses = self.stats.bypasses.saturating_add(1);
            return None;
        }
        let Some(cost) = charge(parsed, self.max_bytes) else {
            self.stats.bypasses = self.stats.bypasses.saturating_add(1);
            return None;
        };
        Some(Pending(Entry {
            work: work.into(),
            cache_version: CACHE_VERSION,
            parser_version: PARSER_VERSION,
            parsed: parsed.clone(),
            cost,
        }))
    }
    /// Only the producer's successfully verified/finalized completed result may
    /// publish a staged parse. Dropping Pending after cancellation has no effect.
    pub(crate) fn commit(&mut self, pending: Pending) {
        let entry = pending.0;
        if !self.enabled || entry.cost > self.max_bytes {
            return;
        }
        self.entries.retain(|old| old.work != entry.work);
        self.charged = self.entries.iter().map(|e| e.cost).sum();
        while self.entries.len() >= self.max_entries
            || self.charged.saturating_add(entry.cost) > self.max_bytes
        {
            let old = self
                .entries
                .pop_front()
                .expect("bounded pending entry fits empty cache");
            self.charged -= old.cost;
        }
        self.charged += entry.cost;
        self.entries.push_back(entry);
    }
}
fn cacheable(snapshot: &SourceSnapshot, parsed: &ParseResult) -> bool {
    if parsed.sources.is_empty()
        || snapshot.inventory.entries.len() > 1000
        || parsed.sources.len() != snapshot.inventory.entries.len()
        || parsed.snapshot_digest != snapshot.digest
        || parsed.candidate_oid != snapshot.binding.candidate_oid
        || parsed
            .sources
            .iter()
            .any(|s| s.status != Terminal::Complete)
        || snapshot
            .inventory
            .sources
            .iter()
            .any(|s| s.status != Terminal::Complete)
    {
        return false;
    }
    let paths: BTreeSet<_> = snapshot.inventory.entries.iter().map(|e| &e.path).collect();
    let parsed_paths: BTreeSet<_> = parsed.sources.iter().map(|e| &e.path).collect();
    paths.len() == snapshot.inventory.entries.len()
        && parsed_paths.len() == parsed.sources.len()
        && paths == parsed_paths
}
fn charge(parsed: &ParseResult, limit: usize) -> Option<usize> {
    struct Counter {
        bytes: usize,
        limit: usize,
    }
    impl std::io::Write for Counter {
        fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
            if b.len() > self.limit.saturating_sub(self.bytes) {
                return Err(std::io::ErrorKind::InvalidInput.into());
            }
            self.bytes += b.len();
            Ok(b.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut counter = Counter { bytes: 0, limit };
    serde_json::to_writer(&mut counter, parsed).ok()?;
    let rows = parsed
        .sources
        .len()
        .saturating_add(parsed.requirements.len())
        .saturating_add(parsed.acceptances.len())
        .saturating_add(parsed.edges.len());
    let cost = counter
        .bytes
        .saturating_mul(2)
        .saturating_add(rows.saturating_mul(512))
        .saturating_add(4096);
    (cost <= limit).then_some(cost)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unknown_versions_and_different_work_never_match() {
        let parsed = ParseResult {
            api_version: crate::model::Version::V1,
            snapshot_digest: String::new(),
            candidate_oid: String::new(),
            sources: vec![],
            requirements: vec![],
            acceptances: vec![],
            edges: vec![],
        };
        let mut entry = Entry {
            work: "frozen".into(),
            cache_version: CACHE_VERSION,
            parser_version: PARSER_VERSION,
            parsed,
            cost: 0,
        };
        assert!(entry.matches("frozen"));
        assert!(!entry.matches("changed"));
        entry.cache_version = "future";
        assert!(!entry.matches("frozen"));
        entry.cache_version = CACHE_VERSION;
        entry.parser_version = "future";
        assert!(!entry.matches("frozen"));
    }
}
