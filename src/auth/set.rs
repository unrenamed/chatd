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

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

    #[test]
    fn test_insert_and_contains_within_expiration() {
        let mut set = TimedHashSet::default();
        let item = "item1";
        let expiration_time = Duration::from_secs(2);

        set.insert(item.to_string(), expiration_time);
        assert!(set.contains(&item.to_string()));
    }

    #[test]
    fn test_insert_and_contains_after_expiration() {
        let mut set = TimedHashSet::default();
        let item = "item2";
        let expiration_time = Duration::from_millis(100);

        set.insert(item.to_string(), expiration_time);
        sleep(Duration::from_millis(150));
        assert!(!set.contains(&item.to_string()));
    }

    #[test]
    fn test_insert_and_iterate_within_expiration() {
        let mut set = TimedHashSet::default();
        let item1 = "item1".to_string();
        let item2 = "item2".to_string();
        let expiration_time = Duration::from_secs(2);

        set.insert(item1.clone(), expiration_time);
        set.insert(item2.clone(), expiration_time);

        let items: Vec<_> = set.iter().cloned().collect();
        assert!(items.contains(&item1));
        assert!(items.contains(&item2));
    }

    #[test]
    fn test_insert_and_iterate_after_expiration() {
        let mut set = TimedHashSet::default();
        let item1 = "item1".to_string();
        let item2 = "item2".to_string();
        let expiration_time = Duration::from_millis(100);

        set.insert(item1.clone(), expiration_time);
        set.insert(item2.clone(), expiration_time);

        sleep(Duration::from_millis(150));

        let items: Vec<_> = set.iter().cloned().collect();
        assert!(!items.contains(&item1));
        assert!(!items.contains(&item2));
    }

    #[test]
    fn test_insert_multiple_items_with_different_expiration_times() {
        let mut set = TimedHashSet::default();
        let item1 = "item1".to_string();
        let item2 = "item2".to_string();
        let short_expiration = Duration::from_millis(100);
        let long_expiration = Duration::from_secs(2);

        set.insert(item1.clone(), short_expiration);
        set.insert(item2.clone(), long_expiration);

        sleep(Duration::from_millis(150));

        assert!(!set.contains(&item1));
        assert!(set.contains(&item2));
    }

    #[test]
    fn test_iter_mixed_expiration_times() {
        let mut set = TimedHashSet::default();
        let item1 = "item1".to_string();
        let item2 = "item2".to_string();
        let short_expiration = Duration::from_millis(100);
        let long_expiration = Duration::from_secs(2);

        set.insert(item1.clone(), short_expiration);
        set.insert(item2.clone(), long_expiration);

        sleep(Duration::from_millis(150));

        let items: Vec<_> = set.iter().cloned().collect();
        assert!(!items.contains(&item1));
        assert!(items.contains(&item2));
    }
}
