use std::clone::Clone;


/// A sliding window that holds items of type T
pub struct SlidingWindow<T> {
    items: Vec<Option<T>>,
    head: usize,    // first index in the vector
    tail: usize,    // last index in the vector
    start: u64,     // first item in the window
}

impl <T> SlidingWindow<T> where T: Clone {

    /// Create a new SlidingWindow with the given capacity
    pub fn new(window_size: usize) -> SlidingWindow<T> {
        let items = vec![None; window_size];

        return SlidingWindow { items, head: 0, tail: 1, start: 0 };
    }

    /// Insert an item at a given location in the window
    /// Any inserts outside of [start, start+window_size) will return None
    /// Otherwise, the value that was in the window position is returned
    pub fn insert(&mut self, loc: u64, item: T) -> Result<(), &str> {
        if loc < self.start {
            return Err("loc < start");
        } else if loc >= self.start + self.items.len() as u64  {
            return Err("loc >= end");
        }

        let index : usize = (loc - self.start) as usize + self.head;

        if self.items[index].is_some() {
            return Err("Value already set");
        }

        // insert the item
        self.items[index] = Some(item);

        // update our tail
        if index >= self.tail {
            self.tail = (index + 1) % self.items.len();
        }

        return Ok( () );
    }

    /// Removes the item at the location
    /// Returns None if there is no item there, and does not slide the window
    pub fn remove(&mut self, loc: u64) -> Result<T, &str> {
        if loc < self.start {
            return Err("loc < start");
        } else if loc >= self.start + self.items.len() as u64  {
            return Err("loc >= end");
        }

        let index : usize = (loc - self.start) as usize + self.head;

        if self.items[index].is_none() {
            return Err("Value not set");
        } else {
            let ret = self.items[index].take().expect("Unwrapped already checked value");

            if index == self.head {
                loop {
                    // update our head and start values
                    self.head = (self.head + 1) % self.items.len();
                    self.start += 1;

                    // keep closing the window, if we're not at the end
                    // and the items are None
                    if self.head == self.tail || self.items[self.head].is_some() {
                        break;
                    }
                }

            }

            return Ok(ret);
        }
    }

    /// Get the [start, end) of the window
    pub fn window(&self) -> (u64, u64) {
        (self.start, self.start + self.items.len() as u64)
    }
}

#[cfg(test)]
mod tests {

    use sliding_window::SlidingWindow;

    #[test]
    fn create_insert() {
        let mut sw = SlidingWindow::<&str>::new(16);

        assert!(sw.insert(3, "hello").is_ok());
        assert!(sw.insert(3, "world").is_err());
        assert!(sw.insert(64, "wrong").is_err());
    }

    #[test]
    fn sender_test() {
        let mut sw = SlidingWindow::<&str>::new(16);

        // insert 3 items in order
        assert!(sw.insert(0, "a").is_ok());
        assert!(sw.insert(1, "b").is_ok());
        assert!(sw.insert(2, "c").is_ok());
        assert_eq!((0,16), sw.window());

        // remove the one in the middle
        assert_eq!(Ok("b"), sw.remove(1));
        assert_eq!((0,16), sw.window());

        // make sure we've incremented twice
        assert_eq!(Ok("a"), sw.remove(0));
        assert_eq!((2,18), sw.window());

        // make sure the window is totally consumed
        assert_eq!(Ok("c"), sw.remove(2));
        assert_eq!((3,19), sw.window());

        // insert items in reverse order
        assert!(sw.insert(5, "f").is_ok());
        assert!(sw.insert(4, "e").is_ok());
        assert!(sw.insert(3, "d").is_ok());
        assert_eq!((3,19), sw.window());

        // remove the one in the middle
        assert_eq!(Ok("e"), sw.remove(4));
        assert_eq!((3,19), sw.window());

        // make sure we've incremented twice
        assert_eq!(Ok("d"), sw.remove(3));
        assert_eq!((5,21), sw.window());

        // make sure the window is totally consumed
        assert_eq!(Ok("f"), sw.remove(5));
        assert_eq!((6,22), sw.window());
    }
}