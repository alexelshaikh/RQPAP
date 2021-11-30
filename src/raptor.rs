use raptorq::{Decoder, Encoder, EncodingPacket, ObjectTransmissionInformation, SourceBlockEncoder};
use crate::dna_rules;
use crate::base_sequence::{BaseSequence, Base};
use std::cmp::{max, min};
use rand::Rng;
use std::rc::Rc;
use rand::rngs::ThreadRng;
use std::ops::{Range, Add, Sub};
use std::sync::Arc;
use std::time::{SystemTime, Duration};

/// The Enum that represents an encoding status of a final DNA strand resembling an Info-DNA.
enum PacketsResult {
    Found(Arc<BaseSequence>, u8),
    RulesNotSatisfied(Arc<BaseSequence>, u8),
    NotDecodable,
    OverheadTooBig(usize)
}

/// RQ's configuration holder.
pub struct RaptorQ {
    source_blocks: usize,
    sub_blocks: usize,
    alignment: usize,
    symbol_size: usize
}

impl RaptorQ {
    /// Creates a new RQ with the given configuration.
    pub fn new(source_blocks: usize, sub_blocks: usize, alignment: usize, symbol_size: usize) -> Self {
        Self { source_blocks, sub_blocks, alignment, symbol_size }
    }
    /// Creates a new RQ with the default configuration.
    pub fn default() -> Self {
        Self { source_blocks: 1, sub_blocks: 1, alignment: 3, symbol_size: 6 }
    }
    /// The function that encodes a data object (in bytes) into an Info-DNA while fulfilling the given DNA constraints. Returns a DNA sequence (Info-DNA) for the given `data`.
    /// # Arguments
    /// * `data` - The data object that is to be encoded to an Info-DNA.
    /// * `packets_per_block` - The number of packets that will be generated initially.
    /// * `max_block_encode_loops` - The number of loops in which we attempt to successfully encode `data`.
    /// * `overhead` - The overhead ε for RQ.
    /// * `gc_and_hp_check` - The function that checks the GC content and homopolymer length requirements for the DNA sequence.
    /// * `strand_rule_no_dg` - The function that checks the constraints on final Info-DNA (excluding the dg error).
    /// * `dg_check` - The function that checks the error by the dg server.
    pub fn encode_to_dna_with_rules(&self,
                                    data: &[u8],
                                    mut packets_per_block: usize,
                                    mut max_block_encode_loops: usize,
                                    overhead: usize,
                                    gc_and_hp_check: impl Fn(&Arc<BaseSequence>) -> bool,
                                    strand_rule_no_dg: impl Fn(&Arc<BaseSequence>) -> bool,
                                    dg_check: impl Fn(&Arc<BaseSequence>) -> bool) -> (Arc<BaseSequence>, Duration, Duration) {

        let start_time = SystemTime::now();
        let mut dg_time = Duration::new(0_u64, 0_u32);
        let encoder = Encoder::new(&data,ObjectTransmissionInformation::new(
            data.len() as u64,
            self.symbol_size as u16,
            self.source_blocks as u8,
            self.sub_blocks as u16,
            self.alignment as u8
        ));

        let source_block_encoder = &encoder.get_block_encoders()[0];
        let mut packets_count = packets_per_block;
        let mut block_loop_num = 0;
        let mut rng = ThreadRng::default();
        let mut last_strand = Arc::new(BaseSequence::empty());
        let mut packets_count_last = 0_u8;
        let mut from_repair_esi = 0_usize;
        let mut good_packets = vec![];
        let mut last_esi = 0_usize;
        while block_loop_num < max_block_encode_loops {
            block_loop_num += 1;
            last_esi = from_repair_esi + packets_count;
            let fresh_packets = Self::generate_packets(source_block_encoder, packets_count, from_repair_esi, &gc_and_hp_check);
            good_packets.extend(fresh_packets);
            for _ in 0..good_packets.len() {
                match Self::combine_packets_to_strand(&good_packets, Decoder::new(encoder.get_config()), overhead, Self::random_order(0..good_packets.len(), &mut rng).as_slice(), &strand_rule_no_dg) {
                    PacketsResult::Found(strand, packets_count) => {
                        let dg_start_time = SystemTime::now();
                        let dg_check_result = dg_check(&strand);
                        dg_time += SystemTime::now().duration_since(dg_start_time).unwrap();
                        if dg_check_result {
                            let rq_time = SystemTime::now().duration_since(start_time).unwrap() - dg_time;
                            return (Self::finalize_encoding(&strand, data.len() as u8, packets_count), rq_time, dg_time);
                        }
                        else {
                            last_strand = strand;
                            packets_count_last = packets_count;
                        }
                    }
                    // the packets could be decodable but do not contain the specified overhead -> need more packets
                    PacketsResult::OverheadTooBig(missing) => {
                        packets_count += missing * packets_per_block + 1_usize;
                        break;
                    }
                    // the packets were not decodable -> need more packets
                    PacketsResult::NotDecodable => {
                        packets_count += packets_per_block;
                        break;
                    }
                    // the packets are decodable but do not meet the requirements given by the constraints
                    PacketsResult::RulesNotSatisfied(strand, packets_count) => {
                        last_strand = strand;
                        packets_count_last = packets_count;
                    }
                }
            }
            from_repair_esi = last_esi + 1;
        }

        (Self::finalize_encoding(&last_strand, data.len() as u8, packets_count_last),
         SystemTime::now().duration_since(start_time).unwrap() - dg_time,
         dg_time)
        //panic!("failed encoding file={:?}", data);
    }


    /// Collects the given `range` into a vector, permutes it by `rng`, and returns the vector.
    #[inline]
    fn random_order(range: Range<usize>, rng: &mut ThreadRng) -> Vec<usize> {
        let count = range.len();
        let mut v = range.collect::<Vec<usize>>();
        for _ in 0..count {
            let i = rng.gen_range(0..count);
            let j = rng.gen_range(0..count);
            let arr_i = v[i];
            v[i] = v[j];
            v[j] = arr_i;
        }

        v
    }

    /// The function that combines `packets` into a single DNA strand. It will opt to combine as many as needed to be decodable and meet the `overhead` specified. The strand must fulfill `strand_id_ok_func`.
    #[inline]
    fn combine_packets_to_strand(packets: &Vec<(Arc<BaseSequence>, Vec<u8>)>, mut decoder: Decoder, overhead: usize, index_order: &[usize], strand_is_ok_func: impl Fn(&Arc<BaseSequence>) -> bool) -> PacketsResult {
        let mut current_overhead = -1_isize;
        let mut decoded = None;
        let mut dna_strand = BaseSequence::new(vec![]);
        let mut packets_used = 0_usize;
        for index in index_order {
            let packet_pair = packets.get(*index).unwrap();
            packets_used += 1;
            decoded = decoder.decode(EncodingPacket::deserialize(packet_pair.1.as_slice()));
            dna_strand.append_slice(packet_pair.0.as_slice());
            if decoded.is_some() {
                current_overhead += 1;
                let missing_packets = (overhead as isize - current_overhead) as isize - (packets.len() - packets_used) as isize;
                if missing_packets > 0 {
                    return PacketsResult::OverheadTooBig(missing_packets as usize);
                }
                if current_overhead >= overhead as isize {
                    let strand_arc = Arc::new(dna_strand);
                    return if strand_is_ok_func(&strand_arc) {
                        PacketsResult::Found(strand_arc, packets_used as u8)
                    } else {
                        PacketsResult::RulesNotSatisfied(strand_arc, packets_used as u8)
                    }
                }
            }
        }
        PacketsResult::NotDecodable
    }

    /// Adds a header (containing the RQ configuration) to `seq` that allows a DNA strand to be decoded.
    #[inline]
    fn finalize_encoding(seq: &Arc<BaseSequence>, data_len: u8, packets_count: u8) -> Arc<BaseSequence> {
        let file_len = Self::map_half_byte_to_bases(data_len);
        let file_packets_count = Self::map_half_byte_to_bases(packets_count);
        let mut final_seq = BaseSequence::concat_slice(file_len.as_slice(), file_packets_count.as_slice());
        final_seq.append_slice(seq.as_slice());
        Arc::new(final_seq)
    }

    /// Generates `packets_per_block` packets that satisfy `rules_func`.
    #[inline]
    pub fn generate_packets(block_encoder: &SourceBlockEncoder, packets_per_block: usize, from_repair_esi: usize, rules_func: impl Fn(&Arc<BaseSequence>) -> bool) -> (Vec<(Arc<BaseSequence>, Vec<u8>)>) {
        let mut packets = Vec::with_capacity(packets_per_block);
        for p in Self::next_n_packets(block_encoder, from_repair_esi, packets_per_block).into_iter() {
            let dna_packet = Arc::new(RaptorQ::map_bytes_to_base_sequence(&p[3..]));
            if rules_func(&dna_packet) {
                packets.push((dna_packet, p));
            }
        }

        packets
    }

    /// Maps a byte slice to a BaseSequence.
    #[inline]
    fn map_bytes_to_base_sequence(slice: &[u8]) -> BaseSequence {
        BaseSequence::new(slice.iter().flat_map(|b| Self::map_byte_to_bases(*b)).collect())
    }

    /// Maps a single byte to 4 DNA bases.
    #[inline]
    fn map_byte_to_bases(b: u8) -> Vec<Base> {
        let mut result = Vec::with_capacity(4);

        result.push(Self::map_byte_to_base((b >> 6) & 0b_0000_0011));
        result.push(Self::map_byte_to_base((b >> 4) & 0b_0000_0011));
        result.push(Self::map_byte_to_base((b >> 2) & 0b_0000_0011));
        result.push(Self::map_byte_to_base(b & 0b_0000_0011));

        result
    }

    /// Maps a half byte to 2 DNA bases.
    #[inline]
    fn map_half_byte_to_bases(b: u8) -> Vec<Base> {
        let mut result = Vec::with_capacity(2);
        result.push(Self::map_byte_to_base((b >> 2) & 0b_0000_0011));
        result.push(Self::map_byte_to_base(b & 0b_0000_0011));

        result
    }


    /// Converts a byte to a single DNA base.
    #[inline]
    fn map_byte_to_base(bits: u8) -> Base {
        unsafe {
            std::mem::transmute(bits)
        }
    }

    /// Computes and returns the next `count` repair packets starting from the encoding symbol id (ESI) `from_repair_esi`.
    #[inline]
    fn next_n_packets(source_block_enc: &SourceBlockEncoder, from_repair_esi :usize, count: usize) -> Vec<Vec<u8>> {
        source_block_enc.repair_packets(from_repair_esi as u32, count as u32).into_iter().map(|p| p.serialize()).collect()
    }
    pub fn source_blocks(&self) -> usize {
        self.source_blocks
    }
    pub fn sub_blocks(&self) -> usize {
        self.sub_blocks
    }
    pub fn alignment(&self) -> usize {
        self.alignment
    }
    pub fn symbol_size(&self) -> usize {
        self.symbol_size
    }
}