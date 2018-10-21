use std::clone::Clone;
use std::sync::{Arc, Mutex};
use std::thread;
use std::sync::atomic::{AtomicUsize, Ordering};

struct SlidingWindowData<T> {
    items: Vec<Option<T>>,
    head: usize,    // first index in the vector
    tail: usize,    // last index in the vector
}

/// A sliding window that holds items of type T
pub struct SlidingWindow<T> {
    start: AtomicUsize,     // first item in the window; TODO: change to AtomicI64
    size: usize,  // size of the window, needed so we can access w/out getting the Mutex
    inner: Mutex<SlidingWindowData<T>>
}

impl <T> SlidingWindow<T> where T: Clone {
    /// Create a new SlidingWindow with the given capacity
    pub fn new(window_size: usize) -> SlidingWindow<T> {
        let items = vec![None; window_size];

        let inner = SlidingWindowData { items, head: 0, tail: 1 };

        SlidingWindow {
            start: AtomicUsize::new(0),
            size: window_size,
            inner: Mutex::new(inner)
        }
    }

    /// Insert an item at a given location in the window
    /// Any inserts outside of [start, start+window_size) will return None
    /// Otherwise, the value that was in the window position is returned
    pub fn insert(&self, loc: u64, item: T) -> Result<(), &str> {
        if loc < self.start.load(Ordering::Acquire) as u64 {
            return Err("loc < start");
        }

        // wait until room is made for this insert
        while loc >= (self.start.load(Ordering::Acquire) + self.size) as u64 {
            thread::yield_now();
        }

        // lock the mutex here
        let mut inner = self.inner.lock().unwrap();

        let index : usize = ((loc as usize - self.start.load(Ordering::Acquire)) + inner.head) % inner.items.len();

        println!("INDEX: {}, LOC: {}, START: {}, HEAD: {}", index, loc, self.start.load(Ordering::Acquire), inner.head);

        if inner.items[index].is_some() {
            return Err("Value already set");
        }

        // insert the item
        inner.items[index] = Some(item);

        // update our tail
        if index >= inner.tail {
            inner.tail = (index + 1) % inner.items.len();
        }

        return Ok( () );
    }

    /// Removes an item in the window, given a location relative to the index
    /// ie, you have to compute loc - start already, and pass that in
    fn inner_remove(&self, relative_loc: u64) -> Option<T> {
        // lock the mutex here
        let mut inner = self.inner.lock().unwrap();

        let index : usize = relative_loc as usize + inner.head;

        if inner.items[index].is_none() {
            return None;
        } else {
            let ret = inner.items[index].take().expect("Unwrapped already checked value");

            if index == inner.head {
                loop {
                    // update our head and start values
                    inner.head = (inner.head + 1) % inner.items.len();
                    self.start.fetch_add(1, Ordering::AcqRel);

                    // keep closing the window, if we're not at the end
                    // and the items are None
                    if inner.head == inner.tail || inner.items[inner.head].is_some() {
                        break;
                    }
                }

            }

            return Some(ret);
        }
    }


    /// Removes the item at the location
    /// Returns None if there is no item there, and does not slide the window
    pub fn remove(&self, loc: u64) -> Result<T, &str> {
        let start = self.start.load(Ordering::Acquire);

        if loc < start as u64 {
            return Err("loc < start");
        } else if loc >= (start + self.size) as u64  {
            return Err("loc >= end");
        }

        match self.inner_remove(loc - start as u64) {
            None => Err("Value is none"),
            Some(t) => Ok(t)
        }
    }

    /// Returns the first element in the window
    /// Saves you from having to do:
    /// let (start, end) = w.window();
    /// let t = w.remove(start);
    pub fn pop(&self) -> T {
        loop {
            let res = self.inner_remove(0);

            if res.is_none() {
                thread::yield_now();
            } else {
                return res.unwrap();
            }
        }
    }

    /// Find the first item in the window that satisfies the predicate
    pub fn find_first<P>(&self, mut predicate: P) -> Option<usize> where P: FnMut(&T) -> bool {
        let inner = self.inner.lock().unwrap();
        let mut cur = inner.head;

        while cur != inner.tail {
            if inner.items[cur].is_some() {
                let item = inner.items[cur].as_ref().unwrap();

                if predicate(item) {
                    return Some(cur + self.start.load(Ordering::Acquire));
                }
            }

            cur = (cur + 1) % self.size; // increment w/wrap
        }

        return None;
    }

    /// Get the [start, end) of the window
    pub fn window(&self) -> (u64, u64) {
        let start :u64 = self.start.load(Ordering::Acquire) as u64;

        (start, start + self.size as u64)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::Duration;

    use sliding_window::SlidingWindow;

    #[test]
    fn create_insert() {
        let mut sw = SlidingWindow::<&str>::new(16);

        assert!(sw.insert(3, "hello").is_ok());
        assert!(sw.insert(3, "world").is_err());
    }

    #[test]
    fn blocking() {
        let sw = Arc::new(SlidingWindow::<&str>::new(3));

        assert!(sw.insert(0, "a").is_ok());
        assert!(sw.insert(1, "b").is_ok());
        assert!(sw.insert(2, "c").is_ok());

        let sw_clone = sw.clone();
        let mut removed = Arc::new(Mutex::new(false));

        let removed_clone = removed.clone();

        thread::spawn(move || {
            thread::sleep(Duration::from_millis(500));
            let mut removed = removed_clone.lock().unwrap();

            sw_clone.remove(0).expect("Error removing item 0");
            *removed = true;
        });

        assert!(sw.insert(3, "d").is_ok());

        if ! *removed.lock().unwrap() {
            panic!("Inserted before removed");
        }
    }

    #[test]
    fn pop_test() {
        let mut sw = SlidingWindow::<&str>::new(16);

        // insert 3 items in order
        assert!(sw.insert(0, "a").is_ok());
        assert!(sw.insert(1, "b").is_ok());
        assert!(sw.insert(2, "c").is_ok());
        assert_eq!((0,16), sw.window());

        assert_eq!("a", sw.pop());
        assert_eq!("b", sw.pop());
        assert_eq!("c", sw.pop());
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