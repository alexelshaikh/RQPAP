use rand::Rng;

#[derive(Eq, PartialEq, Clone, Copy, Debug, Hash)]
pub struct PseudoPermutation {
    m: usize,
    p: usize,
    a: usize,
    b: usize
}

impl PseudoPermutation {
    pub fn new(m: usize) -> Self {
        Self::new_from_p(m, m)
    }

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