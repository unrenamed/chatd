use circular_buffer::CircularBuffer;

#[derive(Debug, Default, Clone)]
pub struct InputHistory<T: Default + Clone, const N: usize> {
    history: CircularBuffer<N, T>,
    nav_index: Option<usize>,
}

impl<T: Default + Clone, const N: usize> InputHistory<T, N> {
    pub fn nav_index(&self) -> Option<usize> {
        self.nav_index
    }

    pub fn push(&mut self, item: T) {
        self.history.push_back(item);
        self.nav_index = None; // Reset navigation index when a new
                               // command is added
    }

    pub fn prev(&mut self) -> Option<&T> {
        if self.history.len() == 0 {
            return None;
        }

        match self.nav_index {
            None => {
                self.nav_index = Some(self.history.len() - 1);
            }
            Some(0) => {}
            Some(index) => {
                self.nav_index = Some(index - 1);
            }
        }

        self.history.get(self.nav_index.unwrap())
    }

    pub fn next(&mut self) -> Option<&T> {
        match self.nav_index {
            None => None,
            Some(index) => {
                if index + 1 >= self.history.len() {
                    self.nav_index = None;
                    None
                } else {
                    self.nav_index = Some(index + 1);
                    self.history.get(self.nav_index.unwrap())
                }
            }
        }
    }

    pub fn insert_at_current(&mut self, item: T) {
        if let Some(index) = self.nav_index {
            if index < self.history.len() {
                self.history[index] = item;
            }
        }
    }
}

#[cfg(test)]
mod should {
    use super::*;

    #[test]
    fn push_item_and_reset_navigation_index() {
        let mut history: InputHistory<String, 3> = InputHistory::default();
        history.push("First".to_string());
        history.push("Second".to_string());

        assert_eq!(history.history.len(), 2);
        assert_eq!(history.history[0], "First".to_string());
        assert_eq!(history.history[1], "Second".to_string());
        assert_eq!(
            history.nav_index(),
            None,
            "Should reset navigation index when a new command is added"
        );
    }

    #[test]
    fn navigate_to_previous_items() {
        let mut history: InputHistory<String, 3> = InputHistory::default();

        assert_eq!(history.prev(), None);
        history.push("First".to_string());
        history.push("Second".to_string());
        assert_eq!(history.prev(), Some(&"Second".to_string()));
        assert_eq!(history.prev(), Some(&"First".to_string()));

        assert_eq!(
            history.prev(),
            Some(&"First".to_string()),
            "Should return the oldest item in the history when navigation index is at the history start"
        );
    }

    #[test]
    fn navigate_to_next_items() {
        let mut history: InputHistory<String, 3> = InputHistory::default();
        history.push("First".to_string());
        history.push("Second".to_string());

        history.prev();
        history.prev();
        assert_eq!(history.next(), Some(&"Second".to_string()));
        assert_eq!(history.next(), None);
        assert_eq!(
            history.next(),
            None,
            "Should return no items when navigation index is at the history end"
        );
    }

    #[test]
    fn modify_item_at_current_navigation_index() {
        let mut history: InputHistory<String, 3> = InputHistory::default();
        history.push("First".to_string());
        history.push("Second".to_string());

        history.prev(); // Move to "Second"
        history.insert_at_current("Modified".to_string());
        assert_eq!(history.history[1], "Modified".to_string());

        history.prev(); // Move to "First"
        history.insert_at_current("Modified First".to_string());
        assert_eq!(history.history[0], "Modified First".to_string());
    }

    #[test]
    fn handle_circular_buffer_behavior() {
        let mut history: InputHistory<String, 3> = InputHistory::default();
        history.push("First".to_string());
        history.push("Second".to_string());
        history.push("Third".to_string());
        history.push("Fourth".to_string());

        assert_eq!(history.history.len(), 3);
        assert_eq!(history.history[0], "Second".to_string());
        assert_eq!(history.history[1], "Third".to_string());
        assert_eq!(history.history[2], "Fourth".to_string());
    }
}
