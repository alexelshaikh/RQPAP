# RaptorQ Probe-Aware Encoding Pipeline (RQPAP)

RQPAP is a software suite that allows encoding arbitrary binary files into DNA. It is used to encode data into a highly scalable DNA key-value storage system.
RQPAP utilizes locality-sensitive hashing (LSH) and RaptorQ code to create sufficiently stable DNA fragments, supporting direct access to each fragment by using a microarray.

## Installation

Make sure you have [Rust](https://www.rust-lang.org/learn/get-started) installed. To build the project, run the following command in the root directory of this project.
```sh
cargo build --release
```
You can run this command on a windows or linux shell, depending on your target machine you want to use. The output executable file will be either `RQPAP/target/release/RQPAP.exe` or `RQPAP/target/release/RQPAP`.




## Usage

The RQPAP requires setting the correct parameters to start encoding. Parameters can be set from the console as such:
`parameter_name=parameter_value`. For example, `lines_path=lines.txt` will set the parameter `lines_path` to `lines.txt`. Furthermore, the probes have to be computed prior to encoding the files. To compute probes, consider running our [Probe Generator](https://github.com/alexelshaikh/PG.git) first.

Note that the RQPAP will list **all** the parameters (with default values if not set). Furthermore, the program will write various time measures to the file `RQPAP_report.csv`, which can be examined after the RQPAP is done. See the next list of parameters to customize the pipeline.

### Parameters:

`lines_path`: path to a file with _n_ **data objects**. Each data object will be encoded to a single DNA fragment.

`read_as_lines`: _true_ to interpret each line of `lines_path` as a data object. _false_ to read the file as follows: 4 bytes will be read (big endian) and converted to an integer _len_. The next _len_ bytes will be interpreted as a data object. RQPAP will loop until it finds the end of the file and report how many data objects it found. This is helpful when you consider encoding, e.g., compressed data objects that may contain the new line character "\n".

`probes_path`: path to a fasta file with _m_ **probes** (usually _m_ = _n_).

`encoding_mode`: Either LSH, MIXED, or NAIVE.
When set to LSH, all similarity checks will get computed with LSH. When set to MIXED, similarity checks between sequences and probes only will be calculated with LSH. Finally, when set to NAIVE, all similarity checks will be calculated without LSH.

`info_dna_path`: path to fasta file to store the encoded files (without probes).

`report_path`: csv file path to which encoding stats will be written to.

`report`: _true_ to enable stats to be written to `report_path` and _false_ to disable writing stats to the csv file.

`report_append`: _true_ to append stats to existing file at `report_path` and _false_ to write to a new file or override the existing one.

`overhead`: epsilon, the redundancy parameter for RQ.

`max_hp_len`: maximum allowed homopolymer length of a sequence.

`min_dist_to_probes`: guaranteed minimum distance of an encoded data object to all the probes.

`min_dist_to_seqs`: guaranteed minimum distance of an encoded data object to all the other encoded data objects.

`lsh_k_probes`: _k_-mer length for LSH used for the LSH instance of the probes.

`lsh_r_probes`: number _r_ of hash functions used for the LSH instance of the probes.

`lsh_b_probes`: number _b_ of bands used for the LSH instance of the probes.

`lsh_k_seqs`: _k_-mer length for LSH instance of the sequences of the data objects.

`lsh_r_seqs`: number _r_ of hash functions used for the LSH instance of the sequences of the data objects.

`lsh_b_seqs`: number _b_ of bands used for the LSH instance of the sequences of the data objects.

`use_dg_server`: _true_ to check for complex secondary structures, else _false_. To enable it, you have to start the python script `server.py` (see below).

### Example

**On Linux** (_RQPAP_)
```sh
./RQPAP lines_path=lines.txt probes_path=probes.fa encoding_mode=LSH
```
**On Windows** (_RQPAP.exe_)


```sh
./RQPAP.exe lines_path=lines.txt probes_path=probes.fa encoding_mode=LSH
```

## Secondary Structure Prediction (`use_dg_server`)

If you wish to set `use_dg_server=true`, you will have to start the [Python 3](https://www.python.org/downloads/) script `server.py` in the directory `dg` beforehand. This script requires [seqfold](https://github.com/Lattice-Automation/seqfold) to be installed. Run the following command to install `seqfold`.
```sh
pip install seqfold
```
Then, to start the server from the root directory of this project, run the following command.
```sh
python dg/server.py
```

The server will automatically start on port 6000. For each additionally available thread, a new port will be used after 6000. For example, if your machine supports 4 threads, the server will use the following ports: 6000, 6001, 6002, and 6003. The RQPAP will use all available ports.

## External Crates Used
Please note that we use the following crates (will automatically get downloaded and installed when building the project).
1. [`parking_lot = "0.11.1"`](https://crates.io/crates/parking_lot)
2. [`num_cpus = "1.13.0"`](https://crates.io/crates/num_cpus)
3. [`rand = "0.8.4"`](https://crates.io/crates/rand)
4. [`raptorq = "1.6.4"`](https://crates.io/crates/raptorq)
5. [`rayon = "1.5.1"`](https://crates.io/crates/rayon)
6. [`crossbeam-channel = "0.5.1"`](https://crates.io/crates/crossbeam-channel)