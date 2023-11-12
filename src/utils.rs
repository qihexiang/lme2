use std::{
    borrow::Borrow,
    collections::{HashMap, HashSet},
    f32::consts::E,
    fmt::Debug,
    hash::Hash,
};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UniqueValueMap<K: Hash + Eq + Clone, V: Hash + Eq + Clone> {
    map: HashMap<K, V>,
    validate: HashSet<V>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InsertResult<K, V> {
    Updated(V),
    Duplicated(K),
    Created,
}

impl<K: Clone + Hash + Eq, V: Clone + Hash + Eq> UniqueValueMap<K, V> {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
            validate: HashSet::new(),
        }
    }

    pub fn from_map(map: HashMap<K, V>) -> Result<Self, HashMap<K, V>> {
        let validate = map.values().cloned().collect::<HashSet<_>>();
        if validate.len() == map.len() {
            Ok(Self { map, validate })
        } else {
            Err(map)
        }
    }

    pub fn data(&self) -> &HashMap<K, V> {
        &self.map
    }

    pub fn insert(&mut self, k: K, v: V) -> InsertResult<K, V> {
        if self.validate.contains(&v) {
            self.map
                .iter()
                .find_map(|(key, value)| if value == &v { Some(key) } else { None })
                .and_then(|key| Some(InsertResult::Duplicated(key.clone())))
                .unwrap()
        } else {
            self.validate.insert(v.clone());
            let result = self
                .map
                .insert(k, v)
                .and_then(|v| Some(InsertResult::Updated(v)))
                .unwrap_or(InsertResult::Created);
            if let InsertResult::Updated(v) = &result {
                self.validate.remove(v);
            }
            result
        }
    }

    pub fn remove<Q: Hash + Eq + ?Sized>(&mut self, k: &Q) -> Option<V>
    where
        K: Borrow<Q>,
    {
        let removed = self.map.remove(k);
        if let Some(value) = &removed {
            self.validate.remove(value);
        }
        removed
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pair<T>(T, T);

impl<T: Eq> Pair<T> {
    pub fn to_tuple(&self) -> (&T, &T) {
        (&self.0, &self.1)
    }
    pub fn get_another(&self, current: &T) -> Option<&T> {
        let Self(a, b) = self;
        if current == a {
            Some(b)
        } else if current == b {
            Some(a)
        } else {
            None
        }
    }

    pub fn contains(&self, current: &T) -> bool {
        self.get_another(current).is_some()
    }
}

impl<T: Ord> Pair<T> {
    pub fn new(a: T, b: T) -> Self {
        let mut values = [a, b];
        values.sort();
        let [a, b] = values;
        Self(a, b)
    }
}

impl<T: Hash> Hash for Pair<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let Self(a, b) = self;
        a.hash(state);
        b.hash(state);
    }
}

#[derive(Debug)]
pub enum LayerRemoveResult<T> {
    Removed(T),
    Shadowed,
    None,
}

pub enum LayerInserResult<'a, T> {
    Created,
    Updated(T),
    Overlayed(&'a T),
}

#[test]
fn uniq_val_map() {
    let mut map: UniqueValueMap<String, usize> = UniqueValueMap::new();
    let x = map.insert("center".to_string(), 1);
    let y = map.insert("center".to_string(), 2);
    let z = map.insert("entry".to_string(), 2);
    assert_eq!(x, InsertResult::Created);
    assert_eq!(y, InsertResult::Updated(1));
    assert_eq!(z, InsertResult::Duplicated("center".to_string()));
    assert_eq!(
        map,
        UniqueValueMap {
            map: HashMap::from([("center".to_string(), 2)]),
            validate: HashSet::from([2])
        }
    )
}

#[test]
fn pair_creation() {
    let pair1 = Pair::new(1, 2);
    let pair2 = Pair::new(1, 2);
    let pair3 = Pair::new(2, 4);
    let pair4 = Pair::new(3, 4);
    let set = HashSet::from([pair1, pair2, pair3, pair4]);
    assert_eq!(
        set,
        HashSet::from([Pair::new(1, 2), Pair::new(2, 4), Pair::new(3, 4)])
    );
    assert_eq!(
        set.into_iter()
            .filter(|pair| pair.contains(&4))
            .collect::<Vec<_>>()
            .len(),
        2
    );
}

pub struct NtoN<L, R>(HashSet<(L, R)>);

impl<L: Eq + Hash + Clone, R: Eq + Hash + Clone> NtoN<L, R> {
    pub fn new() -> Self {
        Self(HashSet::new())
    }

    pub fn data(&self) -> &HashSet<(L, R)> {
        &self.0
    }

    pub fn get_left(&self, left: &L) -> Vec<&R> {
        self.0
            .iter()
            .filter_map(|(l, r)| if l == left { Some(r) } else { None })
            .collect()
    }

    pub fn get_right(&self, right: &R) -> Vec<&L> {
        self.0
            .iter()
            .filter_map(|(l, r)| if r == right { Some(l) } else { None })
            .collect()
    }

    pub fn insert(&mut self, left: L, right: R) -> bool {
        self.0.insert((left, right))
    }

    pub fn remove(&mut self, left: &L, right: &R) -> bool {
        self.0.remove(&(left.clone(), right.clone()))
    }

    pub fn remove_left(&mut self, left: &L) {
        self.0.retain(|(l, _)| l != left)
    }

    pub fn remove_right(&mut self, right: &R) {
        self.0.retain(|(_, r)| r != right)
    }
}

impl<K,V> From<HashSet<(K,V)>> for NtoN<K, V> {
    fn from(value: HashSet<(K,V)>) -> Self {
        Self(value)
    }
}

impl<K,V> Into<HashSet<(K,V)>> for NtoN<K,V> {
    fn into(self) -> HashSet<(K,V)> {
        self.0
    }
}