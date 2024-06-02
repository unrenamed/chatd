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
        self.nav_index = None; // Reset navigation index when a new command is added
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
