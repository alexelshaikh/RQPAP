use std::{env, fs};
use std::time::{SystemTime, Duration};
use std::sync::Arc;
use crate::lsh::LSH;
use crate::raptor::RaptorQ;
use crate::safe_cell::SafeCell;
use std::fs::{OpenOptions, File, read};
use std::io::{BufReader, Read, BufRead, Write, stdout, stdin};
use crate::base_sequence::BaseSequence;
use crate::dg_client::DGClient;
use rayon::ThreadPool;
use crossbeam_channel::{Sender, Receiver, bounded};
use std::ops::{Deref, Add};
use std::path::Path;
use parking_lot::RwLockReadGuard;
use std::collections::HashSet;
use parking_lot::RwLock;

mod lsh;
mod pseudo_permutation;
mod safe_cell;
mod arg_parser;
mod base_sequence;
mod dna_rules;
mod raptor;
mod dg_client;

static DISTANCE_CHECK_POOLING_TRIGGER: usize  = 2000_usize;
static DEFAULT_CSV_DELIMITER: &str            = ",";
static DEFAULT_CSV_NEW_LINE: &str             = "\n";

static ENCODING_MODE_LSH: usize               = 0_usize;
static ENCODING_MODE_MIXED: usize             = 1_usize;
static ENCODING_MODE_NAIVE: usize             = 2_usize;

static INITIAL_PACKETS_PER_BLOCK: usize       = 5_usize;
static MAX_ENCODE_LOOPS: usize                = 200_usize;


static DEFAULT_MAX_ERR: f64                   = 0.5_f64;
static DEFAULT_MAX_HP_LEN: usize              = 5_usize;
static DEFAULT_OVERHEAD: usize                = 0_usize;
static DEFAULT_SECONDARY_STRUCT_TEMP: f32     = 25_f32;
static DEFAULT_MAX_DG_ERROR: f32              = 0.5_f32;
static DEFAULT_DG_START_PORT: u16             = 6000_u16;
static DEFAULT_USE_DG: bool                   = true;
static DEFAULT_READ_AS_LINES: bool            = true;
static DEFAULT_APPROVE: bool                  = true;
static DEFAULT_APPEND_TO_REPORT: bool         = true;
static DEFAULT_REPORT: bool                   = true;
static DEFAULT_REPORT_PATH: &str              = "RQPAP_report.csv";
static DEFAULT_ENCODING_MODE_STR: &str        = "lsh";
static DEFAULT_PROBES_PATH: &str              = "probes.fa";
static DEFAULT_LINES_PATH: &str               = "lines.txt";
static DEFAULT_INFO_DNA_PATH: &str            = "info-dna.fa";

static DEFAULT_LSH_K_PROBES: usize            = 4_usize;
static DEFAULT_LSH_R_PROBES: usize            = 200_usize;
static DEFAULT_LSH_B_PROBES: usize            = 20_usize;

static DEFAULT_LSH_K_SEQS: usize              = 5_usize;
static DEFAULT_LSH_R_SEQS: usize              = 200_usize;
static DEFAULT_LSH_B_SEQS: usize              = 20_usize;

static DEFAULT_MIN_DIST_TO_PROBES: f64        = 0.4_f64;
static DEFAULT_MIN_DIST_TO_SEQS: f64          = 0.4_f64;



fn main() {
    let n_workers = num_cpus::get();
    let args_parser = arg_parser::ArgsParser::from(env::args().skip(1).collect());
    let lines_path = args_parser.get_or_else("lines_path", DEFAULT_LINES_PATH);
    let probes_path = args_parser.get_or_else("probes_path", DEFAULT_PROBES_PATH);
    let info_dna_path = args_parser.get_or_else("info_dna_path", DEFAULT_INFO_DNA_PATH);
    let encoding_mode_str = args_parser.get_or_else("encoding_mode", DEFAULT_ENCODING_MODE_STR);
    let overhead = args_parser.get_as("overhead", DEFAULT_OVERHEAD);
    let max_hp_len = args_parser.get_as("max_hp_len", DEFAULT_MAX_HP_LEN);
    let use_dg_server = args_parser.get_as_bool("use_dg_server", DEFAULT_USE_DG);
    let read_as_lines = args_parser.get_as("read_as_lines", DEFAULT_READ_AS_LINES);
    let approve = args_parser.get_as_bool("approve", DEFAULT_APPROVE);

    let append_to_report = args_parser.get_as_bool("append_to_report", DEFAULT_APPEND_TO_REPORT);
    let report = args_parser.get_as_bool("report", DEFAULT_REPORT);
    let report_path = args_parser.get_or_else("report_path", DEFAULT_REPORT_PATH);

    let min_dist_to_probes = args_parser.get_as("min_dist_to_probes", DEFAULT_MIN_DIST_TO_PROBES);
    let min_dist_to_seqs = args_parser.get_as("min_dist_to_seqs", DEFAULT_MIN_DIST_TO_SEQS);

    let lsh_k_probes = args_parser.get_as("lsh_k_probes", DEFAULT_LSH_K_PROBES);
    let lsh_r_probes = args_parser.get_as("lsh_r_probes", DEFAULT_LSH_R_PROBES);
    let lsh_b_probes = args_parser.get_as("lsh_b_probes", DEFAULT_LSH_B_PROBES);

    let lsh_k_seqs = args_parser.get_as("lsh_k_seqs", DEFAULT_LSH_K_SEQS);
    let lsh_r_seqs = args_parser.get_as("lsh_r_seqs", DEFAULT_LSH_R_SEQS);
    let lsh_b_seqs = args_parser.get_as("lsh_b_seqs", DEFAULT_LSH_B_SEQS);


    let mut encoding_mode = extract_encoding_mode(encoding_mode_str.as_str());

    print_parameters(
        lines_path.as_str(),
        probes_path.as_str(),
        info_dna_path.as_str(),
        overhead,
        max_hp_len,
        read_as_lines,
        use_dg_server,
        encoding_mode_str.as_str(),
        min_dist_to_probes,
        min_dist_to_seqs,
        approve,
        report,
        report_path.as_str(),
        append_to_report,
        encoding_mode,
        lsh_k_probes,
        lsh_r_probes,
        lsh_b_probes,
        lsh_k_seqs,
        lsh_r_seqs,
        lsh_b_seqs);

    if approve && !approve_parameters() {
        println!("------------------------------------------------------");
        println!("-> Parameters were not approved -> program terminated.");
        return;
    }
    println!("------------------------------------------------------");

    let dg_client = Arc::new(match use_dg_server {
        true => match DGClient::new(127, 0, 0, 1, DEFAULT_DG_START_PORT, n_workers as u16) {
            Some(client) => Some(client),
            _ => panic!("failed to connect to dg server!")
        },
        false => None
    });

    let mut lines = read_lines_arc(lines_path.as_str(), read_as_lines);
    println!("lines imported         = {}", lines.len());

    let probes = Arc::new(SafeCell::new(BaseSequence::read_fasta_arc(probes_path.as_str())));
    println!("probes imported        = {}", probes.len());
    println!("------------------------------------------------------");

    let mut probes_lsh = Arc::new(SafeCell::new(LSH::new(lsh_k_probes, 1, 1)));
    let mut seqs_lsh = Arc::new(RwLock::new(SafeCell::new(LSH::new(lsh_k_seqs, 1, 1))));
    let mut start_time = SystemTime::now();
    if encoding_mode == ENCODING_MODE_LSH || encoding_mode == ENCODING_MODE_MIXED {
        println!("building LSH for probes...");
        probes_lsh = Arc::new(SafeCell::new(LSH::new(lsh_k_probes, lsh_r_probes, lsh_b_probes)));
        let start_building_time = SystemTime::now();
        let insert_pool = rayon::ThreadPoolBuilder::new().num_threads(n_workers).build().unwrap();
        let probes_count = probes.len();
        let (sender, receiver) = bounded(probes_count);

        for p in probes.iter() {
            let sender_cloned = sender.clone();
            let probes_lsh_cloned = probes_lsh.clone();
            let probe = p.clone();
            insert_pool.spawn(move|| {
                probes_lsh_cloned.get_mut().insert(&probe);
                sender_cloned.send(true);
            });
        }
        receiver.iter().take(probes_count).for_each(|_| {});
        println!("finished building LSH for probes in {} seconds", SystemTime::now().duration_since(start_building_time).unwrap().as_millis() as f64 / 1000_f64);
    }
    if encoding_mode == ENCODING_MODE_LSH {
        seqs_lsh = Arc::new(RwLock::new(SafeCell::new(LSH::new(lsh_k_seqs, lsh_r_seqs, lsh_b_seqs))));
    }

    println!("initiating...");

    match fs::remove_file(info_dna_path.as_str()) {
        Ok(_) => println!("Overriding file: {}", info_dna_path.as_str()),
        Err(_) => {}
    }
    let mut info_dna_file = OpenOptions::new().append(true).create(true).open(info_dna_path.as_str()).unwrap();
    encode_pipeline(
        n_workers,
        report,
        append_to_report,
        report_path.as_str(),
        use_dg_server,
        probes_lsh,
        seqs_lsh,
        probes,
        info_dna_file,
        lines,
        encoding_mode,
        overhead,
        max_hp_len,
        min_dist_to_probes,
        min_dist_to_seqs,
        dg_client
    );

    let time_millis = SystemTime::now().duration_since(start_time).unwrap().as_millis();
    println!("finished encoding all lines in {} millis", time_millis);
    println!("finished encoding all lines in {} seconds", (time_millis as f64 / 1000 as f64));
    println!("finished encoding all lines in {} minutes", (time_millis as f64 / 1000 as f64 / 60 as f64));
    println!("finished encoding all lines in {} hours", (time_millis as f64 / 1000 as f64 / 60 as f64 / 60 as f64 ));

}


fn encode_pipeline(n_workers: usize,
                   report: bool,
                   append_to_report: bool,
                   report_path: &str,
                   use_dg_server: bool,
                   probes_lsh: Arc<SafeCell<LSH>>,
                   seqs_lsh: Arc<RwLock<SafeCell<LSH>>>,
                   probes: Arc<SafeCell<Vec<Arc<BaseSequence>>>>,
                   mut info_dna_file: File,
                   lines: Vec<Arc<Vec<u8>>>,
                   encoding_mode: usize,
                   overhead: usize,
                   max_hp_len: usize,
                   min_dist_to_probes: f64,
                   min_dist_to_seqs: f64,
                   dg_client: Arc<Option<DGClient>>) {

    if lines.len() != probes.get().len() {
        println!("WARNING: jobs ({}) != probes ({})", lines.len(), probes.get().len());
    }


    let mut csv = None;

    if report {
        if !append_to_report {
            fs::remove_file(report_path);
            csv = Some(OpenOptions::new().append(true).create(true).open(report_path).unwrap());
            csv.as_ref().unwrap().write_all(["Progress(%)", "Line Id", "Done Id", "Trials", "Time(ms)", "Time For", "File Size", "Total Bytes", "Overhead", "Length", "Max HP Length", "Min. Dist To Probes", "Min. Dist To Seqs", "Encoding Mode", "Use DG Server"].join(DEFAULT_CSV_DELIMITER).as_bytes());
        }
        else {
            csv = Some(OpenOptions::new().append(true).create(true).open(report_path).unwrap());
            if Path::new(report_path).metadata().unwrap().len() == 0_u64 {
                csv.as_ref().unwrap().write_all(["Progress(%)", "Line Id", "Done Id", "Trials", "Time(ms)", "Time For", "File Size", "Total Bytes", "Overhead", "Length", "Max HP Length", "Min. Dist To Probes", "Min. Dist To Seqs", "Encoding Mode", "Use DG Server"].join(DEFAULT_CSV_DELIMITER).as_bytes());
            }
        }
    }

    let pool = rayon::ThreadPoolBuilder::new().num_threads(n_workers).build().unwrap();
    let dist_pool = Arc::new(RwLock::new(rayon::ThreadPoolBuilder::new().num_threads(n_workers).build().unwrap()));

    let (sender, receiver) = bounded(lines.len());
    let raptor = Arc::new(RaptorQ::default());
    let mut seqs = Arc::new(RwLock::new(Vec::with_capacity(lines.len())));

    println!("---> [started] <---");

    for line_id in 0..lines.len() {
        let sender_cloned = sender.clone();
        let line = lines.get(line_id).unwrap().clone();
        let raptor_cloned = raptor.clone();
        let encoded_seqs_lsh_cloned = seqs_lsh.clone();
        let probes_lsh_cloned = probes_lsh.clone();
        let dg_client_cloned = dg_client.clone();
        let seqs_cloned = seqs.clone();
        let probes_cloned = probes.clone();
        let dist_pool_cloned = dist_pool.clone();
        pool.spawn(move|| {
            encode_file(
                encoding_mode,
                dist_pool_cloned,
                (line_id + 1_usize, line),
                raptor_cloned,
                encoded_seqs_lsh_cloned,
                probes_lsh_cloned,
                seqs_cloned,
                probes_cloned,
                min_dist_to_probes,
                min_dist_to_seqs,
                sender_cloned,
                INITIAL_PACKETS_PER_BLOCK,
                overhead,
                max_hp_len,
                dg_client_cloned
            )
        });
    }

    let encoding_mode_string = if encoding_mode == ENCODING_MODE_LSH {
        String::from("LSH")
    }
    else if encoding_mode == ENCODING_MODE_MIXED {
        String::from("Mixed")
    }
    else {
        String::from("Naive")
    };

    let use_dg_server_string = use_dg_server.to_string();
    let min_dist_to_probes_string = min_dist_to_probes.to_string();
    let min_dist_to_seqs_string = min_dist_to_seqs.to_string();
    let overhead_string = overhead.to_string();
    let max_hp_length_string = max_hp_len.to_string();
    let mut caption = String::new();
    let mut total_bytes = 0_usize;
    for done_id in 1..=lines.len() {
        let (line_id, seq, trails, size, rq_time, dg_time, total_time) = receiver.recv().unwrap();
        caption.push_str(">");
        caption.push_str((line_id + 1_usize).to_string().as_str());
        BaseSequence::append_to_fasta_file_with_caption_arc(&mut info_dna_file, &seq, caption.as_str(), done_id == 1);
        caption.clear();

        if report {
            total_bytes += size;
            let progress_string = (100_f64 * done_id as f64 / lines.len() as f64).to_string();
            let line_id_string = line_id.to_string();
            let done_id_str = done_id.to_string();
            let trails_string = trails.to_string();
            let rq_time_str = rq_time.to_string();
            let dg_time_str = dg_time.to_string();
            let total_time_string = total_time.to_string();
            let file_size_string = size.to_string();
            let total_bytes_string = total_bytes.to_string();
            let seq_len_string = seq.len().to_string();
            report_to_csv(&mut csv,
                          encoding_mode_string.as_str(),
                          use_dg_server_string.as_str(),
                          min_dist_to_probes_string.as_str(),
                          min_dist_to_seqs_string.as_str(),
                          overhead_string.as_str(),
                          progress_string.as_str(),
                          line_id_string.as_str(),
                          done_id_str.as_str(),
                          trails_string.as_str(),
                          rq_time_str.as_str(),
                          dg_time_str.as_str(),
                          total_time_string.as_str(),
                          file_size_string.as_str(),
                          total_bytes_string.as_str(),
                          seq_len_string.as_str(),
                          max_hp_length_string.as_str());
        }
    }

    if report {
        csv.as_ref().unwrap().flush();
    }

    println!("---> [finished] <---");
}

#[inline(always)]
fn report_to_csv(csv: &mut Option<File>, encoding_mode_string: &str, use_dg_server_string: &str, min_dist_to_probes_string: &str, min_dist_to_seqs_string: &str, overhead_string: &str, progress_string: &str, line_id_string: &str, done_id_str: &str, trails_string: &str, rq_time_str: &str, dg_time_str: &str, total_time_string: &str, file_size_string: &str, total_bytes_string: &str, seq_len_string: &str, max_hp_length_string: &str) {
    let mut row = String::new();
    row.push_str(DEFAULT_CSV_NEW_LINE);
    row.push_str(progress_string);               // progress in %
    row.push_str(DEFAULT_CSV_DELIMITER);
    row.push_str(line_id_string);                // line id
    row.push_str(DEFAULT_CSV_DELIMITER);
    row.push_str(done_id_str);             // done_id
    row.push_str(DEFAULT_CSV_DELIMITER);
    row.push_str(trails_string);                 // trys
    row.push_str(DEFAULT_CSV_DELIMITER);
    row.push_str(rq_time_str);             // rq time
    row.push_str(DEFAULT_CSV_DELIMITER);
    row.push_str("RQ");                    // time type
    row.push_str(DEFAULT_CSV_DELIMITER);
    row.push_str(file_size_string);              // file size
    row.push_str(DEFAULT_CSV_DELIMITER);
    row.push_str(total_bytes_string);            // total bytes
    row.push_str(DEFAULT_CSV_DELIMITER);
    row.push_str(overhead_string);               // overhead
    row.push_str(DEFAULT_CSV_DELIMITER);
    row.push_str(seq_len_string);                // length
    row.push_str(DEFAULT_CSV_DELIMITER);
    row.push_str(max_hp_length_string);          // max hp length
    row.push_str(DEFAULT_CSV_DELIMITER);
    row.push_str(min_dist_to_probes_string);     // min dist to probes
    row.push_str(DEFAULT_CSV_DELIMITER);
    row.push_str(min_dist_to_seqs_string);       // min dist to seqs
    row.push_str(DEFAULT_CSV_DELIMITER);
    row.push_str(encoding_mode_string);          // encoding mode
    row.push_str(DEFAULT_CSV_DELIMITER);
    row.push_str(use_dg_server_string);          // use_dg_server


    row.push_str(DEFAULT_CSV_NEW_LINE);
    row.push_str(progress_string);               // progress in %
    row.push_str(DEFAULT_CSV_DELIMITER);
    row.push_str(line_id_string);                // line id
    row.push_str(DEFAULT_CSV_DELIMITER);
    row.push_str(done_id_str);             // done_id
    row.push_str(DEFAULT_CSV_DELIMITER);
    row.push_str(trails_string);                 // trys
    row.push_str(DEFAULT_CSV_DELIMITER);
    row.push_str(dg_time_str);             // dg time
    row.push_str(DEFAULT_CSV_DELIMITER);
    row.push_str("Sec. Struct.");          // time type
    row.push_str(DEFAULT_CSV_DELIMITER);
    row.push_str(file_size_string);              // file size
    row.push_str(DEFAULT_CSV_DELIMITER);
    row.push_str(total_bytes_string);            // total bytes
    row.push_str(DEFAULT_CSV_DELIMITER);
    row.push_str(overhead_string);               // overhead
    row.push_str(DEFAULT_CSV_DELIMITER);
    row.push_str(seq_len_string);                // length
    row.push_str(DEFAULT_CSV_DELIMITER);
    row.push_str(max_hp_length_string);          // max hp length
    row.push_str(DEFAULT_CSV_DELIMITER);
    row.push_str(min_dist_to_probes_string);     // min dist to probes
    row.push_str(DEFAULT_CSV_DELIMITER);
    row.push_str(min_dist_to_seqs_string);       // min dist to seqs
    row.push_str(DEFAULT_CSV_DELIMITER);
    row.push_str(encoding_mode_string);          // encoding mode
    row.push_str(DEFAULT_CSV_DELIMITER);
    row.push_str(use_dg_server_string);          // use_dg_server


    row.push_str(DEFAULT_CSV_NEW_LINE);
    row.push_str(progress_string);               // progress in %
    row.push_str(DEFAULT_CSV_DELIMITER);
    row.push_str(line_id_string);                // line id
    row.push_str(DEFAULT_CSV_DELIMITER);
    row.push_str(done_id_str);             // done_id
    row.push_str(DEFAULT_CSV_DELIMITER);
    row.push_str(trails_string);                 // trys
    row.push_str(DEFAULT_CSV_DELIMITER);
    row.push_str(total_time_string);             // total time
    row.push_str(DEFAULT_CSV_DELIMITER);
    row.push_str("Total");                 // time type
    row.push_str(DEFAULT_CSV_DELIMITER);
    row.push_str(file_size_string);              // file size
    row.push_str(DEFAULT_CSV_DELIMITER);
    row.push_str(total_bytes_string);            // total bytes
    row.push_str(DEFAULT_CSV_DELIMITER);
    row.push_str(overhead_string);               // overhead
    row.push_str(DEFAULT_CSV_DELIMITER);
    row.push_str(seq_len_string);                // length
    row.push_str(DEFAULT_CSV_DELIMITER);
    row.push_str(max_hp_length_string);          // max hp length
    row.push_str(DEFAULT_CSV_DELIMITER);
    row.push_str(min_dist_to_probes_string);     // min dist to probes
    row.push_str(DEFAULT_CSV_DELIMITER);
    row.push_str(min_dist_to_seqs_string);       // min dist to seqs
    row.push_str(DEFAULT_CSV_DELIMITER);
    row.push_str(encoding_mode_string);          // encoding mode
    row.push_str(DEFAULT_CSV_DELIMITER);
    row.push_str(use_dg_server_string);          // use_dg_server

    csv.as_ref().unwrap().write_all(row.as_bytes());
}


#[inline(always)]
fn dg_error(dg: f32) -> f32 {
    let err = 1_f32 / (1_f32 + f32::exp(dg + 4_f32));
    return if err.is_normal() {
        err
    }
    else {
        0_f32
    }
}

#[inline(always)]
fn encode_file(encoding_mode: usize,
               dist_pool: Arc<RwLock<ThreadPool>>,
               line: (usize, Arc<Vec<u8>>),
               raptor_cloned: Arc<RaptorQ>,
               encoded_seqs_lsh: Arc<RwLock<SafeCell<LSH>>>,
               probes_lsh: Arc<SafeCell<LSH>>,
               seqs: Arc<RwLock<Vec<Arc<BaseSequence>>>>,
               probes: Arc<SafeCell<Vec<Arc<BaseSequence>>>>,
               min_dist_to_probes: f64,
               min_dist_to_seqs: f64,
               sender: Sender<(usize, Arc<BaseSequence>, usize, usize, u128, u128, u128)>,
               packets_per_block: usize,
               overhead: usize,
               max_hp_len: usize,
               dg_client: Arc<Option<DGClient>>) {

    let start_time = SystemTime::now();
    let mut trails = 0_usize;
    let mut result_seq = Arc::new(BaseSequence::empty());
    let seqs_k = encoded_seqs_lsh.read().k();
    let probes_k = probes_lsh.k();
    let dist_pool_cloned = dist_pool.clone();

    let gc_and_hp_check = |seq: &Arc<BaseSequence>| dna_rules::satisfy_gc_hp_rules(seq, max_hp_len);
    let dg_rule = |seq: &Arc<BaseSequence>| dg_error(dg_arc(seq, &dg_client)) <= DEFAULT_MAX_DG_ERROR;
    let strand_func_lsh_mixed_modes = |seq: &Arc<BaseSequence>|
        dna_rules::satisfy_gc_hp_rules(seq, max_hp_len)
            && pooled_dist_check_set(&seq, probes_lsh.similar_seqs(seq), min_dist_to_probes, seqs_k, &dist_pool_cloned);

    let strand_func_naive_mode = |seq: &Arc<BaseSequence>| dna_rules::satisfy_gc_hp_rules(&seq, max_hp_len);

    let mut rq_time_total = Duration::new(0_u64, 0_u32);
    let mut dg_time_total = Duration::new(0_u64, 0_u32);

    loop {
        trails += 1_usize;
        if encoding_mode == ENCODING_MODE_LSH {
            let (encoded_seq, rq_time, dg_time) = raptor_cloned.encode_to_dna_with_rules(
                line.1.as_slice(),
                packets_per_block,
                MAX_ENCODE_LOOPS,
                overhead,
                gc_and_hp_check,
                strand_func_lsh_mixed_modes,
                dg_rule);

            dg_time_total += dg_time;
            rq_time_total += rq_time;
            let time_at_arrival = SystemTime::now();
            let mut write_lock = encoded_seqs_lsh.write();
            if pooled_dist_check_set(&encoded_seq, write_lock.similar_seqs(&encoded_seq), min_dist_to_seqs, seqs_k, &dist_pool) {
                write_lock.insert(&encoded_seq);
                result_seq = encoded_seq;
                rq_time_total += SystemTime::now().duration_since(time_at_arrival).unwrap();
                break;
            }
        }
        else if encoding_mode == ENCODING_MODE_MIXED {
            let (encoded_seq, rq_time, dg_time) = raptor_cloned.encode_to_dna_with_rules(
                line.1.as_slice(),
                packets_per_block,
                MAX_ENCODE_LOOPS,
                overhead,
                gc_and_hp_check,
                strand_func_lsh_mixed_modes,
                dg_rule);

            dg_time_total += dg_time;
            rq_time_total += rq_time;
            let time_at_arrival = SystemTime::now();
            let read_lock = seqs.read();
            let len = read_lock.len();
            if pooled_dist_check(&encoded_seq, read_lock.as_slice(), min_dist_to_seqs, seqs_k, &dist_pool) {
                drop(read_lock);
                if is_inserted_consistent(len, seqs_k, min_dist_to_seqs, seqs.clone(), &encoded_seq, &dist_pool) {
                    result_seq = encoded_seq;
                    rq_time_total += SystemTime::now().duration_since(time_at_arrival).unwrap();
                    break;
                }
            }
        }
        else {
            let (encoded_seq, rq_time, dg_time) = raptor_cloned.encode_to_dna_with_rules(
                line.1.as_slice(),
                packets_per_block,
                MAX_ENCODE_LOOPS,
                overhead,
                gc_and_hp_check,
                strand_func_naive_mode,
                dg_rule);

            dg_time_total += dg_time;
            rq_time_total += rq_time;
            let time_at_arrival = SystemTime::now();
            let read_lock = seqs.read();
            let len = read_lock.len();
            if pooled_dist_check(&encoded_seq, read_lock.as_slice(), min_dist_to_seqs, seqs_k, &dist_pool)
            && pooled_dist_check(&encoded_seq, probes.as_slice(), min_dist_to_probes, probes_k, &dist_pool) {
                drop(read_lock);
                if is_inserted_consistent(len, seqs_k, min_dist_to_seqs, seqs.clone(), &encoded_seq, &dist_pool) {
                    result_seq = encoded_seq;
                    rq_time_total += SystemTime::now().duration_since(time_at_arrival).unwrap();
                    break;
                }
            }
        }
    }

    sender.send((
        line.0,
        result_seq,
        trails,
        line.1.len(),
        rq_time_total.as_millis(),
        dg_time_total.as_millis(),
        SystemTime::now().duration_since(start_time).unwrap().as_millis()));
}


#[inline(always)]
fn is_inserted_consistent(len: usize, k: usize, min_dist_to_seqs: f64, seqs: Arc<RwLock<Vec<Arc<BaseSequence>>>>, encoded_seq: &Arc<BaseSequence>, dist_pool: &Arc<RwLock<ThreadPool>>) -> bool {
    let mut write_lock = seqs.write();
    let diff = write_lock.len() - len;
    if diff == 0_usize {
        write_lock.push(encoded_seq.clone());
        return true;
    }
    else {
        if pooled_dist_check(encoded_seq, &write_lock[len..], min_dist_to_seqs, k, dist_pool) {
            write_lock.push(encoded_seq.clone());
            return true;
        }
    }

    false
}

#[inline(always)]
pub fn extract_encoding_mode(arg: &str) -> usize {
    return if arg.eq_ignore_ascii_case("lsh") {
        ENCODING_MODE_LSH
    }
    else if arg.eq_ignore_ascii_case("naive") {
        ENCODING_MODE_NAIVE
    }
    else if arg.eq_ignore_ascii_case("mixed") {
        ENCODING_MODE_MIXED
    }
    else {
        panic!("cannot determine encoding style: {}", arg);
    }
}


#[inline(always)]
pub fn dg_arc(seq: &Arc<BaseSequence>, dg_client: &Arc<Option<DGClient>>) -> f32 {
    match dg_client.as_ref() {
        None => 0_f32,
        Some(client) => client.dg_arc(seq, DEFAULT_SECONDARY_STRUCT_TEMP)
    }
}

#[inline(always)]
fn read_lines_arc(lines_path: &str, read_as_lines: bool) -> Vec<Arc<Vec<u8>>> {
    if read_as_lines {
        let file = OpenOptions::new().read(true).open(lines_path).unwrap();
        let reader = BufReader::new(file);
        reader.lines().map(|c| Arc::new(c.unwrap().into_bytes())).collect()
    }
    else {
        let mut br = BufReader::new(OpenOptions::new().read(true).open(lines_path).unwrap());
        let mut buff_size = [0_u8; 4];
        let mut lines = vec![];
        loop  {
            match br.read_exact(&mut buff_size) {
                Ok(_) => {
                    let size = u32::from_be_bytes(buff_size);
                    let mut buff_entry = Vec::with_capacity(size as usize);
                    unsafe { buff_entry.set_len(size as usize) };
                    br.read_exact(&mut buff_entry).unwrap_or_else(|e| panic!("wrong len. Err={:?}", e));
                    lines.push(Arc::new(buff_entry));
                }
                Err(_) => {
                    break;
                }
            }
        }
        lines
    }
}

fn approve_parameters() -> bool {
    let mut s= String::new();
    print!("\nAre these parameters correct? [y/n]\n");
    stdout().flush();
    stdin().read_line(&mut s).expect("Did not enter a correct string");
    if let Some('\n') = s.chars().next_back() {
        s.pop();
    }
    if let Some('\r') = s.chars().next_back() {
        s.pop();
    }

    s.eq_ignore_ascii_case("y") || s.eq_ignore_ascii_case("1") || s.eq_ignore_ascii_case("yes") || s.eq_ignore_ascii_case("true")
}

#[inline(always)]
fn pooled_dist_check(seq: &Arc<BaseSequence>, candidates: &[Arc<BaseSequence>], min: f64, k: usize, pool: &Arc<RwLock<ThreadPool>>) -> bool {
    if candidates.len() < DISTANCE_CHECK_POOLING_TRIGGER {
        for candidate in candidates.iter() {
            if seq.jaccard_distance_arc(candidate, k) < min  {
                return false;
            }
        }
        return true
    }
    let is_dist_ok = Arc::new(parking_lot::RwLock::new(true));
    let (tx, rx) = bounded(candidates.len());
    let seq_arc = Arc::new(seq.clone());
    let pool_lock = pool.write();
    for candidate in candidates.iter() {
        let is_dist_ok_cloned = is_dist_ok.clone();
        let sender = tx.clone();
        let s = seq_arc.clone();
        let can = candidate.clone();
        pool_lock.spawn(move|| {
            if *is_dist_ok_cloned.read() {
                sender.send(s.jaccard_distance_arc(&can, k));
            }
        });
    }
    for _ in 0..candidates.len() {
        if rx.recv().unwrap() < min {
            *is_dist_ok.write() = false;
            return false
        }
    }

    true
}


fn pooled_dist_check_set(seq: &Arc<BaseSequence>, candidates: HashSet<Arc<BaseSequence>>, min: f64, k: usize, pool: &Arc<RwLock<ThreadPool>>) -> bool {
    if candidates.len() < DISTANCE_CHECK_POOLING_TRIGGER {
        for candidate in candidates.iter() {
            if seq.jaccard_distance_arc(candidate, k) < min  {
                return false;
            }
        }
        return true
    }
    let is_dist_ok = Arc::new(parking_lot::RwLock::new(true));
    let (tx, rx) = bounded(candidates.len());
    let seq_arc = Arc::new(seq.clone());
    let pool_lock = pool.write();
    for candidate in candidates.iter() {
        let is_dist_ok_cloned = is_dist_ok.clone();
        let sender = tx.clone();
        let s = seq_arc.clone();
        let can = candidate.clone();
        pool_lock.spawn(move|| {
            if *is_dist_ok_cloned.read() {
                sender.send(s.jaccard_distance_arc(&can, k));
            }
        });
    }
    for _ in 0..candidates.len() {
        if rx.recv().unwrap() < min {
            *is_dist_ok.write() = false;
            return false;
        }
    }
    true
}


#[inline(always)]
fn print_parameters(lines_path: &str,
                    probes_path: &str,
                    info_dna_path: &str,
                    overhead: usize,
                    max_hp_len: usize,
                    read_as_lines: bool,
                    use_dg_server: bool,
                    encoding_mode_str: &str,
                    min_dist_to_probes: f64,
                    min_dist_to_seqs: f64,
                    approve: bool,
                    report: bool,
                    report_path: &str,
                    append_to_report: bool,
                    encoding_mode: usize,
                    lsh_k_probes: usize,
                    lsh_r_probes: usize,
                    lsh_b_probes: usize,
                    lsh_k_seqs: usize,
                    lsh_r_seqs: usize,
                    lsh_b_seqs: usize) {

    println!("++++++++++++++++++++++++++++++++");
    println!("-> Using following parameters <-");
    println!("++++++++++++++++++++++++++++++++");
    println!("lines_path             = {}", &lines_path);
    println!("probes_path            = {}", &probes_path);
    if Path::new(info_dna_path).exists() {
        println!("info_dna_path          = {} [file will be overridden]", info_dna_path);
    }
    else {
        println!("info_dna_path          = {}", info_dna_path);
    }
    println!("overhead               = {}", overhead);
    println!("max_hp_len             = {}", max_hp_len);
    println!("read_as_lines          = {}", read_as_lines);
    println!("use_dg_server          = {}", use_dg_server);
    println!("encoding_mode          = {}", encoding_mode_str);
    println!("min_dist_to_probes     = {}", min_dist_to_probes);
    println!("min_dist_to_seqs       = {}", min_dist_to_seqs);
    println!("approve                = {}", approve);
    println!("report                 = {}", report);
    if report {
        println!("append_to_report       = {}", append_to_report);
        println!("report_path            = {}", report_path);
    }
    else {
        println!("append_to_report       = {} [ignored]", append_to_report);
        println!("report_path            = {} [ignored]", report_path);
    }

    if encoding_mode == ENCODING_MODE_LSH {
        println!("lsh_k_probes           = {}", lsh_k_probes);
        println!("lsh_r_probes           = {}", lsh_r_probes);
        println!("lsh_b_probes           = {}", lsh_b_probes);
        println!("lsh_k_seqs             = {}", lsh_k_seqs);
        println!("lsh_r_seqs             = {}", lsh_r_seqs);
        println!("lsh_b_seqs             = {}", lsh_b_seqs);
    }
    else if encoding_mode == ENCODING_MODE_MIXED {
        println!("lsh_k_probes           = {}", lsh_k_probes);
        println!("lsh_r_probes           = {}", lsh_r_probes);
        println!("lsh_b_probes           = {}", lsh_b_probes);
        println!("lsh_k_seqs             = {} [ignored]", lsh_k_seqs);
        println!("lsh_r_seqs             = {} [ignored]", lsh_r_seqs);
        println!("lsh_b_seqs             = {} [ignored]", lsh_b_seqs);
    }
    else {
        println!("lsh_k_probes           = {} [ignored]", lsh_k_probes);
        println!("lsh_r_probes           = {} [ignored]", lsh_r_probes);
        println!("lsh_b_probes           = {} [ignored]", lsh_b_probes);
        println!("lsh_k_seqs             = {} [ignored]", lsh_k_seqs);
        println!("lsh_r_seqs             = {} [ignored]", lsh_r_seqs);
        println!("lsh_b_seqs             = {} [ignored]", lsh_b_seqs);
    }
}