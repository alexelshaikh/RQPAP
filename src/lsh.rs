use std::collections::{HashMap, HashSet};
use crate::base_sequence::{BaseSequence, Base};
use crate::pseudo_permutation::PseudoPermutation;
use std::collections::hash_map::RandomState;
use std::sync::Arc;
use parking_lot::{RwLock, Mutex, RawRwLock};
use std::hash::Hash;
use std::ops::{DerefMut, Deref};
use crate::safe_cell::SafeCell;

pub struct LSH {
    k: usize,
    band_size: usize,
    bands: Vec<RwLock<HashMap<String, HashSet<Arc<BaseSequence>>>>>,
    permutations: Vec<PseudoPermutation>
}

impl LSH {
    /// Creates an LSH instance that is completely thread-safe.
    /// # Arguments
    /// * `k` - The length of the k-mers.
    /// * `r` - The number of hash functions.
    /// * `b` - The number of bands.
    pub fn new(k: usize, r: usize, b: usize) -> Self {
        if r % b != 0_usize {
            panic!("r must be a multiple of b");
        }
        if k > 33_usize {
            panic!("this LSH only supports k-mers up to k = 33");
        }

        let k_mers = 4_usize.pow(k as u32);
        let mut p = k_mers;
        let mut ps = Vec::with_capacity(r);
        for _ in 0..r {
            let permutation = PseudoPermutation::new_from_p(k_mers, p);
            p = permutation.get_p();
            ps.push(permutation);
        }

        LSH {
            k,
            band_size: r / b,
            bands: (0..b).map(|_| RwLock::new(HashMap::new())).collect::<Vec<_>>(),
            permutations: ps
        }
    }

    /// Inserts `seq` into the LSH.
    pub fn insert(&mut self, seq: &Arc<BaseSequence>) {
        let sigs = self.signatures(seq);
        for band in 0_usize..self.bands.len() {
            let sig = sigs[band].as_str();
            let mut map = self.bands.get_mut(band).unwrap().write();
            match map.get_mut(sig) {
                None => {
                    let mut set = HashSet::new();
                    set.insert(seq.clone());
                    map.insert(sig.to_owned(), set);
                }
                Some(set) => {
                    set.insert(seq.clone());
                }
            }
        }
    }

    /// Queries the LSh with `seq` and returns similar sequence it matches.
    pub fn similar_seqs(&self, seq: &Arc<BaseSequence>) -> HashSet<Arc<BaseSequence>> {
        let sigs = self.signatures(seq);
        let mut result = HashSet::new();
        for band in 0_usize..self.bands.len() {
            match self.bands[band].read().get(sigs[band].as_str()) {
                Some(set) => {
                    set.iter().for_each(|s| {
                        result.insert(s.clone());
                    })
                },
                None => {}
            };

        }
        result
    }

    pub fn min_hashes(&self, seq: &Arc<BaseSequence>) -> Vec<usize> {
        let mut min_hashes = Vec::with_capacity(self.permutations.len());
        let mut min_hash:usize;
        let mut perm_hash:usize;
        for i in 0_usize..self.permutations.len() {
            let p = &self.permutations[i];
            min_hash = usize::MAX;
            for shingle in seq.k_mers(self.k).into_iter().map(|k_mer| Self::initial_row_id(k_mer)).collect::<Vec<_>>() {
                perm_hash = p.apply(shingle);
                if perm_hash == 0_usize {
                    min_hash = 0_usize;
                    break;
                }
                if perm_hash < min_hash {
                    min_hash = perm_hash;
                }
            }
            min_hashes.push(min_hash);

        }
        min_hashes
    }

    pub fn signatures(&self, seq: &Arc<BaseSequence>) -> Vec<String> {
        let min_hashes = self.min_hashes(seq);
        let mut sigs = Vec::with_capacity(self.bands.len());
        let mut offset = 0_usize;

        for _ in 0_usize..self.bands.len() {
            let mut sb = String::new();
            for m in 0..self.band_size {
                sb.push_str(min_hashes[m + offset].to_string().as_str());
            }

            sigs.push(sb);
            offset = offset + self.band_size;
        }

        sigs
    }

    pub fn initial_row_id(seq: &[Base]) -> usize {
        let mut id = 0_usize;
        for i in 0_usize..seq.len() {
            let order = match seq[i] {
                Base::A => 0,
                Base::C => 1,
                Base::G => 2,
                Base::T => 3
            };
            if order == 0_usize {
                continue;
            }

            id = id + order * 4_usize.pow(i as u32);
        }

        id
    }


    #[inline]
    pub fn k(&self) -> usize {
        self.k
    }

    #[inline]
    pub fn band_size(&self) -> usize {
        self.band_size
    }
}