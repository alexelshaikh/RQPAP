use crate::base_sequence::BaseSequence;
use std::sync::Arc;


const MIN_GC_CONTENT: f64 = 0.40;
const MAX_GC_CONTENT: f64 = 0.60;

/// Checks if a sequence `seq` satisfies the given constraints on the GC content and maximum homopolymer length.
pub fn satisfy_gc_hp_rules(seq: &Arc<BaseSequence>, max_hp_len: usize) -> bool {
    let gc = seq.gc();
    gc >= MIN_GC_CONTENT && gc <= MAX_GC_CONTENT && seq.longest_hp() <= max_hp_len
}