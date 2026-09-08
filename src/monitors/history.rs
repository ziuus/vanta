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

#[cfg(test)]
mod tests {
    use super::History;

    #[test]
    fn recent_returns_oldest_first_and_only_real_samples() {
        let mut h = History::<4>::new();
        assert!(h.recent(10).is_empty());
        h.push(1.0);
        h.push(2.0);
        assert_eq!(h.recent(10), vec![1.0, 2.0]);
        h.push(3.0);
        h.push(4.0);
        h.push(5.0); // wraps, drops 1.0
        assert_eq!(h.recent(10), vec![2.0, 3.0, 4.0, 5.0]);
        assert_eq!(h.recent(2), vec![4.0, 5.0]);
    }
}
