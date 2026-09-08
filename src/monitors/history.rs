/// Fixed-capacity ring buffer of samples, oldest → newest.
pub struct History<const N: usize> {
    buf: [f64; N],
    head: usize,
    len: usize,
}

impl<const N: usize> History<N> {
    pub const fn new() -> Self {
        Self {
            buf: [0.0; N],
            head: 0,
            len: 0,
        }
    }

    pub fn push(&mut self, v: f64) {
        self.buf[self.head] = v;
        self.head = (self.head + 1) % N;
        self.len = (self.len + 1).min(N);
    }

    /// Up to `n` most recent samples, oldest first. Only real samples are
    /// returned, so a young history renders as a short right-aligned trace
    /// instead of a flat line of zeros.
    pub fn recent(&self, n: usize) -> Vec<f64> {
        let take = n.min(self.len);
        (0..take)
            .map(|i| self.buf[(self.head + N - take + i) % N])
            .collect()
    }
}
