use std::clone::Clone;


/// A sliding window that holds items of type T
pub struct SlidingWindow<T> {
    items: Vec<Option<T>>,
    head: usize, // the first index in the vector
    start: u64 // the first item in the window
}

impl <T> SlidingWindow<T> where T: Clone {

    /// Create a new SlidingWindow with the given capacity
    pub fn new(window_size: usize) -> SlidingWindow<T> {
        let items = vec![None; window_size];

        return SlidingWindow { items, head: 0, start: 0 };
    }

    /// Insert an item at a given location in the window
    /// Any inserts outside of [start, start+window_size) will return None
    /// Otherwise, the value that was in the window position is returned
    pub fn insert(&mut self, loc: u64, item: T) -> Option<T> {
        if loc < self.start || loc >= self.start + (self.items.len() as u64) {
            return None;
        }

        let index : usize = (loc - self.start) as usize + self.head;
        let ret = self.items[index].take();

        self.items[index] = Some(item);

        return ret;
    }

    /// Removes the first item in the sliding window, and slides the window
    /// Returns None if there is no item there, and does not slide the window
    pub fn pop(&mut self) -> Option<T> {
        if self.items[self.head].is_none() {
            return None;
        } else {
            let ret = self.items[self.head].take();

            // update our head and start values
            self.head = (self.head + 1) % self.items.len();
            self.start += 1;

            return ret;
        }
    }

    pub fn window(&self) -> (u64, u64) {
        (self.start, self.start + self.items.len() as u64)
    }
}

#[cfg(test)]
mod tests {

    use sliding_window::SlidingWindow;

    #[test]
    fn create_insert() {
        let mut sw = SlidingWindow::<&str>::new(64);

        assert_eq!(None, sw.insert(3, "hello"));
        assert_eq!(Some("hello"), sw.insert(3, "world"));
        assert_eq!(None, sw.insert(64, "wrong"));
    }

    #[test]
    fn pop() {
        let mut sw = SlidingWindow::<&str>::new(64);

        assert_eq!(None, sw.insert(0, "hello"));
        assert_eq!(None, sw.insert(1, "world"));

        assert_eq!(Some("hello"), sw.pop());

        let w = sw.window();
        println!("{} -> {}", w.0, w.1);

        assert_eq!(Some("world"), sw.pop());

        let w = sw.window();
        println!("{} -> {}", w.0, w.1);

        assert_eq!(None, sw.pop());

        let w = sw.window();
        println!("{} -> {}", w.0, w.1);
    }
}