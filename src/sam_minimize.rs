
use crate::common::{parse_args, GzipWriter, open_bam};
use std::str;
use std::collections::HashMap;
use rust_htslib::bam;
use rust_htslib::bam::{Read, header::Header};
use rust_htslib::bam::record::Record;

const USAGE: &str = "
Usage:
  sam minimize [options] <bam_file>

Options:
  --uncompressed    Output in uncompressed BAM format
  --read-ids        Minimize read identifiers (i.e. QNAME fields)
  --base-qualities  Remove per-base qualities
  --tags            Remove all aux fields (tags)

Changes read IDs into simple numeric identifiers, removes per-base qualities,
and removes all auxiliary fields (tags).
";

pub fn main() {
	let args = parse_args(USAGE);
	let bam_path = args.get_str("<bam_file>");
	let minimize_qnames = args.get_bool("--read-ids");
	let remove_baseq = args.get_bool("--base-qualities");
	let remove_tags = args.get_bool("--tags");

	if !minimize_qnames && !remove_baseq && !remove_tags {
		error!("One of --read-ids, --base-qualities, or --tags must be given.");
	}

	if remove_baseq && !remove_tags {
		error!("Running 'sam minimize' with --base-qualities but without the --tags flag is not yet supported.");
	}

	let mut bam = open_bam(bam_path);
	let header = bam.header().clone();

	// These variables are used for QNAME simplification
	let mut highest_id: u32 = 0;
	let mut qname_to_id: HashMap<Vec<u8>, u32> = HashMap::new();

	let mode: &[u8] = match args.get_bool("--uncompressed") {
		true => b"wbu", false => b"wb"
	};
	//let mut out = bam::Writer::from_stdout(&Header::from_template(&header)).unwrap();
	let mut out = bam::Writer::new(b"-", mode,
		&Header::from_template(&header)).unwrap();

	for r in bam.records() {
		let mut read = r.unwrap_or_else(
			|_| error!("Input BAM file ended prematurely."));

		let mut qname = read.qname().to_vec();

		if minimize_qnames {
			if let Some(slash_pos) = qname.iter().position(|x| *x == b'/') {
				qname.truncate(slash_pos);
			}

			let id = if let Some(x) = qname_to_id.remove(&qname) { x } else {
				highest_id += 1;
				qname_to_id.insert(qname.clone(), highest_id);
				highest_id
			};
			qname = format!("{}", id).into_bytes();
		}

		let cigar = read.cigar();
		let seq = read.seq().as_bytes();

		let qual = if remove_baseq {
			// According to BAM specification, missing per-base qualities
			// are denoted with 0xFF bytes (number must equal sequence length)
			vec![0xFFu8; seq.len()]
		} else {
			read.qual().to_vec()
		};

		if minimize_qnames && !remove_baseq && !remove_tags {
			read.set_qname(&qname);
		} else {
			// The call to .set() removes all AUX fields
			read.set(&qname, &cigar, &seq, &qual);
		}
		out.write(&read).unwrap();
	}
}
