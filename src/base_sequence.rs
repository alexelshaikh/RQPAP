use crate::base_sequence::Base::{A, C, G, T};
use std::iter::FromIterator;
use std::fs;
use std::fs::{OpenOptions, File};
use std::io::Write;
use std::sync::{Mutex, Arc};
use rand::Rng;
use std::collections::HashSet;

/// The Enum that represents a DNA base.
#[derive(Eq, PartialEq, Clone, Copy, Debug, Hash)]
#[repr(u8)]
pub enum Base {
    A = 0,
    C = 1,
    G = 2,
    T = 3,
}

impl Base {
    pub const ALL: [Base; 4] = [A, C, G, T];

    /// Returns the complement for a given DNA base.
    pub fn complement(&self) -> Self {
        match self {
            A => T,
            G => C,
            T => A,
            C => G
        }
    }

    /// Converts a base to a string.
    pub fn to_string(&self) -> &str {
        match self {
            A => "A",
            G => "G",
            T => "T",
            _ => "C"
        }
    }

    /// Parses an ascii byte into a DNA base.
    pub fn from_byte(b: &u8) -> Self {
        match b {
            b'A' => A,
            b'C' => C,
            b'G' => G,
            _ => T
        }
    }

    /// Returns true if the base is a C or a G.
    pub const fn is_c_or_g(&self) -> bool {
        match self {
            C | G => true,
            _ => false
        }
    }

    /// Returns a DNA base. `gc_content` is the probability of returning a C or a G.
    pub fn random_gc(gc_content: f64) -> Self {
        let rand = rand::thread_rng().gen_range(0_f64..1_f64);
        let gs = gc_content / 2 as f64;
        let a = gc_content + 0.5_f64 - gs;

        if rand <= gs {
            G
        }
        else if rand <= gc_content {
            C
        }
        else if rand <= a {
            A
        }
        else {
            T
        }
    }

    /// Returns a random DNA base.
    pub fn random() -> Self {
        let rand = rand::thread_rng().gen_range(0_f64..1_f64);
        if rand <= 0.25_f64 {
            A
        }
        else if rand <= 0.5_f64 {
            T
        }
        else if rand <= 0.75_f64 {
            C
        }
        else {
            G
        }
    }
}

/// The representation for a DNA sequence as a vector or DNA bases.
#[derive(Eq, PartialEq, Clone, Debug, Hash)]
pub struct BaseSequence {
    sequence: Vec<Base>
}


impl BaseSequence {
    pub fn new(sequence: Vec<Base>) -> Self {
        Self {
            sequence
        }
    }

    /// Clears the content of the vector of bases, i.e., returns the sequence empty.
    pub fn clear(&mut self) {
        self.sequence.clear();
    }

    /// Creates a new BaseSequence by parsing a slice of DNA bases.
    pub fn from_slice(slice: &[Base]) -> Self {
        Self::new(slice.to_vec())
    }

    /// Appends the given BaseSequence `seq` to the current one.
    pub fn append_seq(&mut self, seq: &BaseSequence) {
        self.append_slice(seq.as_slice())
    }

    /// Appends the given slice of DNA bases `slice` to the current BaseSequence.
    #[inline]
    pub fn append_slice(&mut self, slice: &[Base]) {
        self.sequence.extend_from_slice(slice)
    }

    /// Returns the k-mers as a vector (duplicates are possible).
    pub fn k_mers(&self, len: usize) -> Vec<&[Base]> {
        if len > self.len() {
            panic!("cannot create kmers of k={} for seq of len {}", len, self.len());
        }
        let size_limit = 1 + self.len() - len;
        let mut kmers = Vec::with_capacity(size_limit);
        for i in 0..size_limit {
            kmers.push(self.sub_sequence_slice(i, i + len));
        }
        kmers
    }

    /// Returns the k-mers as a set (duplicates are not possible).
    pub fn k_mers_set(&self, len: usize) -> HashSet<&[Base]> {
        if len > self.len() {
            panic!("cannot create kmers of k={} for seq of len {}", len, self.len());
        }

        (0..1 + self.len() - len).map(|i| self.sub_sequence_slice(i, i + len)).collect::<HashSet<_>>()
    }

    /// Reads a fasta file with DNA sequences into a vector of BaseSequence.
    pub fn read_fasta_arc(file_path: &str) -> Vec<Arc<BaseSequence>> {
        fs::read_to_string(file_path).iter().flat_map(|s| s.split('\n')).filter(|l| !l.starts_with('>') && l.len() > 0).map(|s| Arc::new(BaseSequence::from_str(s))).collect()
    }


    /// Appends a given sequence `seq` to the fasta file `file`. `is_first_entry` denotes whether or not `file` is empty.
    pub fn append_to_fasta_file_with_caption_arc(file: &mut File, seq: &Arc<BaseSequence>, caption: &str, is_first_entry: bool) {
        let mut entry = if is_first_entry {
            String::with_capacity(caption.len() + 1 + seq.len())
        }
        else {
            let mut s = String::with_capacity(caption.len() + 2 + seq.len());
            s.push_str("\n");
            s
        };

        entry.push_str(caption);
        entry.push_str("\n");
        entry.push_str(seq.to_string().as_str());
        file.write_all(entry.as_bytes());
        file.flush();
    }

    /// Creates a new BaseSequence by concatinating the two given sloces of DNA bases together.
    pub fn concat_slice(slice_1: &[Base], slice_2: &[Base]) -> BaseSequence {
        let mut result_seq
            = BaseSequence::new(Vec::with_capacity(slice_1.len() + slice_2.len()));

        result_seq.sequence.extend_from_slice(slice_1);
        result_seq.sequence.extend_from_slice(slice_2);
        result_seq
    }

    /// Creates a new BaseSequence by parsing the given string `str`.
    pub fn from_str(str: &str) -> Self {
        BaseSequence {
            sequence: str.as_bytes().iter().map(|b| Base::from_byte(b)).collect()
        }
    }

    /// Creates a new empty BaseSequence.
    pub fn empty() -> Self {
        BaseSequence {
            sequence: vec![]
        }
    }

    /// Returns a slice of the current Basesequence beginning at `start` and ending at `end`.
    #[inline(always)]
    pub fn sub_sequence_slice(&self, start: usize, end: usize) -> &[Base] {
        &self.sequence[start..end]
    }

    /// Returns a slice of the current BaseSequence.
    #[inline]
    pub fn as_slice(&self) -> &[Base] {
        &self.sequence
    }


    /// Returns the count of `search_seq` in the current BaseSequence. If the parameter `consecutive` is set true, only consecutive repeats of `search_seq` will be counted.
    pub fn search_count(&self, search_seq: &BaseSequence, consecutive: bool) -> usize {
        BaseSequence::search_count_of(self.as_slice(), search_seq.as_slice(), consecutive)
    }

    /// Returns the count of `search_seq` in the `source`. If the parameter `consecutive` is set true, only consecutive repeats of `search_seq` will be counted.
    #[inline(always)]
    pub fn search_count_of(source: &[Base], search_seq: &[Base], consecutive: bool) -> usize {
        let m_slice = source;
        let slice = search_seq;

        if m_slice.len() < slice.len() {
            return 0;
        }

        let mut start = 0;
        let mut end = slice.len();
        let mut count = 0;
        let mut max_consecutive_count = 0;
        let mut consecutive_count = 0;

        while end < m_slice.len() {
            if m_slice[start..end].eq(slice) {
                count += 1;
                consecutive_count += 1;
                start += slice.len();
                end += slice.len();
            } else {
                consecutive_count = 0;

                start += 1;
                end += 1;
            }

            max_consecutive_count = max_consecutive_count.max(consecutive_count);
        }

        if consecutive {
            max_consecutive_count
        } else {
            count
        }
    }


    /// Returns a string representation as DNA bases for the current BaseSequence.
    pub fn to_string(&self) -> String {
        self.sequence.iter().map(|b| b.to_string()).collect()
    }

    /// Returns the number of DNA bases in the current BaseSequence.
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.sequence.len()
    }


    /// Calculates the Jaccard distance of the current BaseSequence to `to` using the k-mer length of the specified `k`.
    #[inline]
    pub fn jaccard_distance_arc(&self, to: &Arc<BaseSequence>, k: usize) -> f64 {
        let my_shingles = self.k_mers_set(k);
        let that_shingles = to.k_mers_set(k);
        let intersection_size = my_shingles.intersection(&that_shingles).count();
        let union_size = my_shingles.union(&that_shingles).count();

        //println!("dist={}", intersection_size as f64 / union_size as f64);
        1_f64 - (intersection_size as f64 / union_size as f64)
    }

    /// Calculates the normalized Edit distance of the current BaseSequence to `to`.
    #[inline]
    pub fn edit_distance_arc(&self, to: &Arc<BaseSequence>) -> f64 {
        let max_len = usize::max(self.len(), to.len());
        self.levenshtein_distance_arc(to, max_len) as f64 / max_len as f64
    }

    /// Calculates the Edit distance of the current BaseSequence to `to`.
    #[inline(always)]
    fn levenshtein_distance_arc(&self, seq: &Arc<BaseSequence>, max_len: usize) -> usize {
        if self.len() == 0 {
            return seq.len();
        }
        if seq.len() == 0 {
            return self.len();
        }

        let mut v0 = Vec::from_iter(0..seq.len() + 1);
        let mut v1 = vec![0; seq.len() + 1];
        for i in 0..self.len() {
            v1[0] = i + 1;
            let mut min_v1 = v1[0];
            for j in 0..seq.len() {
                let mut cost = 1;
                if self.sequence[i] == seq.sequence[j] {
                    cost = 0;
                }
                v1[j + 1] = usize::min(v1[j] + 1, usize::min(v0[j + 1] + 1, v0[j] + cost));
                min_v1 = usize::min(min_v1, v1[j + 1]);
            }
            if min_v1 >= max_len {
                return max_len;
            }

            let v_temp = v0;
            v0 = v1;
            v1 = v_temp;
        }

        v0[seq.len()]
    }

    /// Returns a new BaseSequence that is the complement of the current BaseSequence.
    #[inline(always)]
    pub fn complement(&self) -> Self {
        Self {
            sequence: self.sequence.iter().map(|base| base.complement()).collect()
        }
    }

    #[inline(always)]
    pub fn gc_of(sequence: &[Base]) -> f64 {
        sequence.iter().filter(|c| c.is_c_or_g()).count() as f64 / sequence.len() as f64
    }

    #[inline(always)]
    pub fn gc(&self) -> f64 {
        Self::gc_of(self.sequence.as_slice())
    }


    /// Returns the length of the longest homopolymer in the current BaseSequence.
    #[inline(always)]
    pub fn longest_hp(&self) -> usize {
        let mut longest = 1;
        let mut current = 1;

        for i in 1..self.len() {
            unsafe {
                if self.sequence.get_unchecked(i - 1) == self.sequence.get_unchecked(i) {
                    current += 1;
                } else {
                    longest = usize::max(current, longest);
                    current = 1;
                }
            }
        }

        usize::max(current, longest)
    }
}





