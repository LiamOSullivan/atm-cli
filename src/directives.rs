// directives.rs
//
// Copyright (c) 2019 All The Music, LLC
//
// This work is licensed under the Creative Commons Attribution 4.0 International License.
// To view a copy of this license, visit http://creativecommons.org/licenses/by/4.0/ or send
// a letter to Creative Commons, PO Box 1866, Mountain View, CA 94042, USA.

extern crate clap;
extern crate libatm;
extern crate pbr;

/****************************/
/***** Single Directive *****/
/****************************/

#[derive(Debug)]
pub struct SingleDirectiveArgs {
    pub sequence: libatm::MIDINoteSequence,
    pub target: String,
}

impl<'a> From<&clap::ArgMatches<'a>> for SingleDirectiveArgs {
    fn from(matches: &clap::ArgMatches<'a>) -> SingleDirectiveArgs {
        // Generate libatm::MIDINoteSequence from notes argument
        let sequence = matches.value_of("NOTES").unwrap();
        let sequence = sequence.parse::<libatm::MIDINoteSequence>().unwrap();

        // Parse target argument
        let target = matches.value_of("TARGET").unwrap().to_string();

        SingleDirectiveArgs { sequence, target }
    }
}

pub fn atm_single(args: SingleDirectiveArgs) {
    println!("::: INFO: Generating MIDI file from pitch sequence");
    // Create MIDIFile from sequence
    let mfile = libatm::MIDIFile::new(args.sequence, libatm::MIDIFormat::Format0, 1, 1);
    println!(
        "::: INFO: Attempting to write MIDI file to path {}",
        &args.target
    );
    // Attempt to write file to target path
    if let Err(err) = mfile.write_file(&args.target) {
        panic!(
            "Failed to write MIDI file to path {} ({})",
            &args.target, err
        );
    } else {
        println!("::: INFO: Successfully wrote MIDI file");
    }
}

/***************************/
/***** Batch Directive *****/
/***************************/

#[derive(Debug)]
pub struct BatchDirectiveArgs {
    pub sequence: libatm::MIDINoteSequence,
    pub length: u32,
    pub target: String,
    pub partition_depth: u32,
    pub max_files: f32,
    pub partition_size: u32,
    pub batch_size: u32,
    pub max_count: usize,
    pub update: u64,
}

impl<'a> From<&clap::ArgMatches<'a>> for BatchDirectiveArgs {
    fn from(matches: &clap::ArgMatches<'a>) -> BatchDirectiveArgs {
        // Generate libatm::MIDINoteSequence from notes argument
        let sequence = matches.value_of("NOTES").unwrap();
        let sequence = sequence.parse::<libatm::MIDINoteSequence>().unwrap();

        // Parse length argument as integer
        let length = matches.value_of("LENGTH").unwrap();
        let length = length.parse::<u32>().unwrap();

        // Target sequence length cannot be less than # of notes
        // TODO: remove this requirement to allow sequences longer than length
        if (length as usize) < sequence.notes.len() {
            panic!(
                "Length must be >= the number of notes in the sequence ({} < {})",
                length,
                sequence.notes.len()
            );
        }

        // Parse target argument
        let target = matches.value_of("TARGET").unwrap().to_string();

        // Parse partition_depth argument as integer
        let partition_depth = matches.value_of("PARTITION_DEPTH").unwrap();
        let partition_depth = partition_depth.parse::<u32>().unwrap();

        // Parse max_files argument and set default if not provided
        let max_files = matches.value_of("MAX_FILES");
        let max_files = match max_files {
            None => 4096.0,
            Some(files) => files.parse::<f32>().unwrap(),
        };

        // Calculate partition size (# of notes) from given arguments (see: gen_partition_size)
        let partition_size = crate::utils::gen_partition_size(
            sequence.notes.len() as f32,
            length as i32,
            max_files,
            partition_depth as i32,
        );

        // Parse max_count argument and set default if not provided
        let max_count = matches.value_of("COUNT");
        let max_count = match max_count {
            None => ((sequence.notes.len() as f32).powi(length as i32) as usize),
            Some(count) => {
                let count = count.parse::<usize>().unwrap();
                if count == 0 {
                    panic!("Count must be greater than 0");
                }
                count
            }
        };

        // Parse batch_size argument
        let batch_size = matches.value_of("BATCH_SIZE").unwrap();
        let batch_size = batch_size.parse::<u32>().unwrap();

        // Parse update argument and set default if not provided
        let update = matches.value_of("PB_UPDATE");
        let update: u64 = match update {
            None => 1000,
            Some(duration) => duration.parse::<u64>().unwrap(),
        };

        BatchDirectiveArgs {
            sequence,
            length,
            target,
            partition_depth,
            max_files,
            partition_size,
            batch_size,
            max_count,
            update,
        }
    }
}

pub fn atm_batch(args: BatchDirectiveArgs) {
    // Initialize progress bar and set refresh rate
    let mut pb = pbr::ProgressBar::new(args.max_count as u64);
    pb.set_max_refresh_rate(Some(std::time::Duration::from_millis(args.update)));
    // Initialize output archive
    let mut archive = crate::utils::BatchedMIDIArchive::new(
        &args.target,
        args.partition_depth,
        args.max_files,
        args.partition_size,
        args.batch_size,
    );
    // For each generated sequence
    for (idx, notes) in crate::utils::gen_sequences(&args.sequence.notes, args.length).enumerate() {
        println!("{}: {:?}", idx + 1, &notes);
        // if reached max count, finish
        if idx == args.max_count {
            archive.finish().unwrap();
            break;
        }
        // Clone libatm::MIDINoteSequence from Vec<&libatm::MIDINote>
        let seq = libatm::MIDINoteSequence::new(
            notes
                .iter()
                .map(|note| *note.clone())
                .collect::<Vec<libatm::MIDINote>>(),
        );
        // Create MIDIFile from libatm::MIDINoteSequence
        let mfile = libatm::MIDIFile::new(seq, libatm::MIDIFormat::Format0, 1, 1);
        // Add MIDIFile to archive
        archive.push(mfile).unwrap();
        // Increment progress bar
        pb.inc();
    }
    // Stop progress bar
    pb.finish_println("");
    // Finish archive if not already finished
    if let crate::utils::BatchedMIDIArchiveState::Open = archive.state {
        archive.finish().unwrap();
    }
}

/***************************/
/***** Partition Directive *****/
/***************************/

#[derive(Debug)]
pub struct PartitionDirectiveArgs {
    pub sequence: libatm::MIDINoteSequence,
    pub partition_depth: u32,
    pub max_files: f32,
    pub partition_size: u32,
}

impl<'a> From<&clap::ArgMatches<'a>> for PartitionDirectiveArgs {
    fn from(matches: &clap::ArgMatches<'a>) -> PartitionDirectiveArgs {
        // Generate libatm::MIDINoteSequence from notes argument
        let sequence = matches.value_of("NOTES").unwrap();
        let sequence = sequence.parse::<libatm::MIDINoteSequence>().unwrap();

        // Parse partition_depth argument as integer
        let partition_depth = matches.value_of("PARTITION_DEPTH").unwrap();
        let partition_depth = partition_depth.parse::<u32>().unwrap();

        // Parse max_files argument and set default if not provided
        let max_files = matches.value_of("MAX_FILES");
        let max_files = match max_files {
            None => 4096.0,
            Some(files) => files.parse::<f32>().unwrap(),
        };

        // Calculate partition size (# of notes) from given arguments (see: gen_partition_size)
        let partition_size = crate::utils::gen_partition_size(
            sequence.notes.len() as f32,
            sequence.notes.len() as i32,
            max_files,
            partition_depth as i32,
        );

        PartitionDirectiveArgs {
            sequence,
            partition_depth,
            max_files,
            partition_size,
        }
    }
}

pub fn atm_partition(args: PartitionDirectiveArgs) {
    println!("::: INFO: Generating MIDI file from pitch sequence");
    // Create MIDIFile from sequence
    let mfile = libatm::MIDIFile::new(args.sequence, libatm::MIDIFormat::Format0, 1, 1);
    // Generate MIDI sequence hash
    let hash = mfile.gen_hash();
    println!("::: INFO: Generating partition(s)");
    // Generate partitions
    let path = crate::utils::gen_path(&hash, args.partition_size, args.partition_depth);
    // Print full path with partitions
    println!("::: INFO: Path for sequence is {}/{}.mid", &path, &hash);
}
