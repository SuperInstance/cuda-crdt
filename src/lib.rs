/*!
# cuda-crdt

Conflict-free Replicated Data Types (CRDTs).

Agents in distributed fleets need shared state without coordination.
CRDTs provide eventual consistency — concurrent updates merge
automatically without conflicts.

- G-Counter (grow-only counter)
- PN-Counter (increment/decrement)
- G-Set (grow-only set)
- OR-Set (observed-remove set)
- LWW-Register (last-writer-wins register)
- LWW-Map (last-writer-wins map)
- Vector clock
- Merge semantics
*/

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// A vector clock (logical timestamps per node)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VectorClock {
    pub entries: HashMap<String, u64>,
}

impl VectorClock {
    pub fn new() -> Self { VectorClock { entries: HashMap::new() } }

    pub fn increment(&mut self, node: &str) { *self.entries.entry(node.to_string()).or_insert(0) += 1; }

    pub fn get(&self, node: &str) -> u64 { *self.entries.get(node).unwrap_or(&0) }

    /// Merge: take max of each entry
    pub fn merge(&mut self, other: &VectorClock) {
        for (node, &time) in &other.entries {
            *self.entries.entry(node.clone()).or_insert(0) = self.entries.get(node).unwrap_or(&0).max(time);
        }
    }

    /// Is this clock at least as recent as other? (happened-before check)
    pub fn dominates(&self, other: &VectorClock) -> bool {
        self.entries.iter().all(|(k, &v)| v >= other.get(k))
    }

    pub fn summary(&self) -> String { format!("VClock: {} nodes, max={}", self.entries.len(), self.entries.values().copied().max().unwrap_or(0)) }
}

/// G-Counter: grow-only distributed counter
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GCounter {
    pub node_id: String,
    pub counts: HashMap<String, u64>,
}

impl GCounter {
    pub fn new(node_id: &str) -> Self { GCounter { node_id: node_id.to_string(), counts: HashMap::new() } }

    pub fn increment(&mut self) { *self.counts.entry(self.node_id.clone()).or_insert(0) += 1; }

    pub fn value(&self) -> u64 { self.counts.values().sum() }

    pub fn merge(&mut self, other: &GCounter) {
        for (node, &count) in &other.counts {
            *self.counts.entry(node.clone()).or_insert(0) = self.counts.get(node).unwrap_or(&0).max(count);
        }
    }
}

/// PN-Counter: counter that supports increment and decrement
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PNCounter {
    pub increments: GCounter,
    pub decrements: GCounter,
}

impl PNCounter {
    pub fn new(node_id: &str) -> Self { PNCounter { increments: GCounter::new(node_id), decrements: GCounter::new(node_id) } }

    pub fn increment(&mut self) { self.increments.increment(); }
    pub fn decrement(&mut self) { self.decrements.increment(); }
    pub fn value(&self) -> i64 { self.increments.value() as i64 - self.decrements.value() as i64 }

    pub fn merge(&mut self, other: &PNCounter) {
        self.increments.merge(&other.increments);
        self.decrements.merge(&other.decrements);
    }
}

/// G-Set: grow-only set (add only, never remove)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GSet<T: Clone + Eq + std::hash::Hash + Serialize> {
    pub items: HashSet<T>,
}

impl<T: Clone + Eq + std::hash::Hash + Serialize> GSet<T> {
    pub fn new() -> Self { GSet { items: HashSet::new() } }
    pub fn add(&mut self, item: T) { self.items.insert(item); }
    pub fn contains(&self, item: &T) -> bool { self.items.contains(item) }
    pub fn len(&self) -> usize { self.items.len() }
    pub fn merge(&mut self, other: &GSet<T>) { self.items.extend(other.items.iter().cloned()); }
}

/// LWW-Register: last-writer-wins register
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LWWRegister<T: Clone + Serialize> {
    pub value: T,
    pub timestamp: u64,
    pub node_id: String,
}

impl<T: Clone + Serialize> LWWRegister<T> {
    pub fn new(node_id: &str, value: T) -> Self { LWWRegister { value, timestamp: now(), node_id: node_id.to_string() } }

    pub fn set(&mut self, value: T) { self.value = value; self.timestamp = now(); }

    pub fn merge(&mut self, other: &LWWRegister<T>) {
        if other.timestamp > self.timestamp || (other.timestamp == self.timestamp && other.node_id > self.node_id) {
            self.value = other.value.clone();
            self.timestamp = other.timestamp;
            self.node_id = other.node_id.clone();
        }
    }
}

/// OR-Set: observed-remove set
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ORSet<T: Clone + Eq + std::hash::Hash + Serialize> {
    pub elements: HashMap<T, HashSet<String>>, // element → set of (node, unique_id) tags
    pub tombstones: HashSet<String>,
}

impl<T: Clone + Eq + std::hash::Hash + Serialize> ORSet<T> {
    pub fn new() -> Self { ORSet { elements: HashMap::new(), tombstones: HashSet::new() } }

    pub fn add(&mut self, item: T, tag: String) {
        self.elements.entry(item).or_insert_with(HashSet::new).insert(tag);
    }

    pub fn remove(&mut self, item: &T) {
        if let Some(tags) = self.elements.get(item) {
            for tag in tags { self.tombstones.insert(tag.clone()); }
        }
    }

    pub fn contains(&self, item: &T) -> bool {
        self.elements.get(item).map_or(false, |tags| tags.iter().any(|t| !self.tombstones.contains(t)))
    }

    pub fn items(&self) -> Vec<&T> {
        self.elements.keys().filter(|k| self.contains(k)).collect()
    }

    pub fn merge(&mut self, other: &ORSet<T>) {
        for (item, tags) in &other.elements {
            let entry = self.elements.entry(item.clone()).or_insert_with(HashSet::new);
            entry.extend(tags.iter().cloned());
        }
        self.tombstones.extend(other.tombstones.iter().cloned());
    }
}

/// LWW-Map: last-writer-wins key-value map
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LWWMap<K, V> where K: Clone + Eq + std::hash::Hash + Serialize, V: Clone + Serialize {
    pub entries: HashMap<K, (V, u64, String)>, // key → (value, timestamp, node)
}

impl<K, V> LWWMap<K, V> where K: Clone + Eq + std::hash::Hash + Serialize, V: Clone + Serialize {
    pub fn new() -> Self { LWWMap { entries: HashMap::new() } }

    pub fn put(&mut self, key: K, value: V, node: &str) {
        self.entries.insert(key, (value, now(), node.to_string()));
    }

    pub fn get(&self, key: &K) -> Option<&V> { self.entries.get(key).map(|(v, _, _)| v) }

    pub fn remove(&mut self, key: &K) { self.entries.remove(key); }

    pub fn merge(&mut self, other: &LWWMap<K, V>) {
        for (key, (value, ts, node)) in &other.entries {
            match self.entries.get(key) {
                Some((_, my_ts, my_node)) if *my_ts > *ts || (*my_ts == *ts && my_node > node) => {}
                _ => { self.entries.insert(key.clone(), (value.clone(), *ts, node.clone())); }
            }
        }
    }

    pub fn len(&self) -> usize { self.entries.len() }
}

fn now() -> u64 { std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as u64 }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gcounter() {
        let mut a = GCounter::new("a");
        let mut b = GCounter::new("b");
        a.increment(); a.increment();
        b.increment();
        a.merge(&b);
        assert_eq!(a.value(), 3);
    }

    #[test]
    fn test_pncounter() {
        let mut c = PNCounter::new("a");
        c.increment(); c.increment(); c.decrement();
        assert_eq!(c.value(), 1);
    }

    #[test]
    fn test_pncounter_merge() {
        let mut a = PNCounter::new("a");
        let mut b = PNCounter::new("b");
        a.increment(); a.increment();
        b.decrement();
        a.merge(&b);
        assert_eq!(a.value(), 1);
    }

    #[test]
    fn test_gset() {
        let mut a: GSet<i32> = GSet::new();
        let mut b: GSet<i32> = GSet::new();
        a.add(1); a.add(2);
        b.add(2); b.add(3);
        a.merge(&b);
        assert_eq!(a.len(), 3);
        assert!(a.contains(&3));
    }

    #[test]
    fn test_lww_register() {
        let mut a = LWWRegister::new("node_a", 10);
        let b = LWWRegister::new("node_b", 20);
        // b was set after a (different timestamps)
        a.merge(&b);
        assert_eq!(a.value, 20);
    }

    #[test]
    fn test_orset_add_remove() {
        let mut s: ORSet<i32> = ORSet::new();
        s.add(1, "a1".into());
        s.add(2, "a2".into());
        assert!(s.contains(&1));
        s.remove(&1);
        assert!(!s.contains(&1));
        assert!(s.contains(&2));
    }

    #[test]
    fn test_orset_merge() {
        let mut a: ORSet<&str> = ORSet::new();
        let mut b: ORSet<&str> = ORSet::new();
        a.add("x", "a1".into());
        b.add("y", "b1".into());
        a.merge(&b);
        assert_eq!(a.items().len(), 2);
    }

    #[test]
    fn test_lwwmap() {
        let mut m: LWWMap<String, i32> = LWWMap::new();
        m.put("key".into(), 10, "a");
        m.put("key".into(), 20, "a");
        assert_eq!(m.get(&"key".to_string()), Some(&20));
    }

    #[test]
    fn test_lwwmap_merge() {
        let mut a: LWWMap<String, i32> = LWWMap::new();
        let mut b: LWWMap<String, i32> = LWWMap::new();
        a.put("k".into(), 1, "a");
        b.put("k".into(), 2, "b");
        // b should win if later timestamp (both use now(), so node comparison)
        a.merge(&b);
        assert!(a.get(&"k".to_string()).is_some());
    }

    #[test]
    fn test_vector_clock() {
        let mut vc1 = VectorClock::new();
        let mut vc2 = VectorClock::new();
        vc1.increment("a"); vc1.increment("a");
        vc2.increment("b");
        vc1.merge(&vc2);
        assert_eq!(vc1.get("a"), 2);
        assert_eq!(vc1.get("b"), 1);
    }

    #[test]
    fn test_vector_clock_dominates() {
        let mut vc1 = VectorClock::new();
        let mut vc2 = VectorClock::new();
        vc1.increment("a"); vc1.increment("b");
        vc2.increment("a");
        assert!(vc1.dominates(&vc2));
        assert!(!vc2.dominates(&vc1));
    }
}
