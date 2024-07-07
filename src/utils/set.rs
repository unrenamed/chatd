use std::collections::HashMap;
use std::time::{Duration, Instant};

#[derive(Debug, Default, Clone)]
pub struct TimedHashSet<T> {
    items: HashMap<T, Instant>,
    expiration_times: HashMap<T, Duration>,
}

impl<T> TimedHashSet<T>
where
    T: Eq + std::hash::Hash + Clone,
{
    pub fn new() -> Self {
        TimedHashSet {
            items: HashMap::new(),
            expiration_times: HashMap::new(),
        }
    }

    pub fn insert(&mut self, item: T, expiration_time: Duration) {
        let now = Instant::now();
        self.items.insert(item.clone(), now);
        self.expiration_times.insert(item, expiration_time);
    }

    pub fn contains(&mut self, item: &T) -> bool {
        if let Some(creation_time) = self.items.get(item) {
            let expiration_time = self.expiration_times.get(item).unwrap();
            if creation_time.elapsed() < *expiration_time {
                true
            } else {
                self.items.remove(item);
                self.expiration_times.remove(item);
                false
            }
        } else {
            false
        }
    }

    pub fn iter(&self) -> TimedHashSetIter<T> {
        TimedHashSetIter {
            items_iter: self.items.iter(),
            expiration_times: &self.expiration_times,
        }
    }
}

pub struct TimedHashSetIter<'a, T> {
    items_iter: std::collections::hash_map::Iter<'a, T, Instant>,
    expiration_times: &'a HashMap<T, Duration>,
}

impl<'a, T> Iterator for TimedHashSetIter<'a, T>
where
    T: Eq + std::hash::Hash + Clone,
{
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some((item, &creation_time)) = self.items_iter.next() {
            if let Some(expiration_time) = self.expiration_times.get(&item) {
                if creation_time.elapsed() < *expiration_time {
                    return Some(item);
                }
            }
        }
        None
    }
}
