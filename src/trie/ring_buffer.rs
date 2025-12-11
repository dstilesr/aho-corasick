/// A Circular buffer to keep the last "capacity" items that have been pushed to it.
#[derive(Debug)]
pub struct RingBuffer<T: Copy> {
    values: Vec<T>,
    capacity: usize,
    pos: usize,
}

impl<T: Copy> RingBuffer<T> {
    /// Instantiate a new buffer with the given capacity.
    pub fn new(capacity: usize) -> Self {
        Self {
            values: Vec::with_capacity(capacity),
            capacity,
            pos: 0,
        }
    }

    /// Push a new element onto the buffer.
    ///
    /// Adds a new element to the ring buffer. If the buffer is at capacity, the
    /// oldest element in it will be removed.
    pub fn push(&mut self, item: T) {
        if self.values.len() < self.capacity {
            self.values.push(item);
        } else {
            self.values[self.pos] = item;
            self.pos = (self.pos + 1) % self.capacity;
        }
    }

    /// The number of elements currently stored on the buffer.
    #[inline]
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Get the item at the given index. Panics if the index is out of bounds.
    pub fn get_item(&self, index: usize) -> T {
        if index >= self.len() {
            panic!("Index out of bounds");
        }
        self.values[(index + self.pos) % self.capacity]
    }
}
