use rand::Rng;

#[derive(Eq, PartialEq, Clone, Copy, Debug, Hash)]
pub struct PseudoPermutation {
    m: usize,
    p: usize,
    a: usize,
    b: usize
}

impl PseudoPermutation {
    /// Creates a PseudoPermutation instance. This is an approximation for a real permutation, and is used to accelerate LSH.
    /// # Arguments
    /// * `m` - The largest index for this instance to permute. For example, if you want to permute a 100 elements vector, m would be 100.
    pub fn new(m: usize) -> Self {
        Self::new_from_p(m, m)
    }

    /// Creates a PseudoPermutation instance. This is an approximation for a real permutation, and is used to accelerate LSH.
    /// # Arguments
    /// * `m` - The largest index for this instance to permute. For example, if you want to permute a 100 elements vector, m would be 100.
    /// * `p_1` - `p_1` must be greater than or equal to `m`. This LSH will use the next prime number greater than `p_1`.
    pub fn new_from_p(m: usize, p_1: usize) -> Self {
        if p_1 < m {
            panic!("p must be >= m");
        }

        let p = Self::next_prime(p_1);
        PseudoPermutation {
            m,
            p: Self::next_prime(p_1),
            a: 1 + rand::thread_rng().gen_range(0..p),
            b: 1 + rand::thread_rng().gen_range(0..p)
        }
    }

    pub fn get_p(&self) -> usize {
        self.p
    }

    /// Permutes index `x` to the permuted index.
    pub fn apply(&self, x: usize) -> usize {
        ((self.a * x + self.b) % self.p) % self.m
    }

    fn next_prime(n: usize) -> usize {
        let mut p = n + 1;
        if (p & 1) == 0 {
            p += 1;
        }
        while !Self::is_odd_number_also_prime(p) {
            p += 2;
        }
        p
    }

    fn is_odd_number_also_prime(p: usize) -> bool {
        (3..=(p as f64).sqrt() as usize).step_by(2).into_iter().all(|n| (p % n) != 0)
    }
}