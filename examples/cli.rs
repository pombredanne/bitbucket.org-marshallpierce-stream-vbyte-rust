extern crate stream_vbyte;
extern crate clap;

use std::io::{BufRead, Read, Write};
use clap::{App, Arg, SubCommand};

fn main() {
    let matches = App::new("stream-vbyte cli")
        .subcommand(SubCommand::with_name("enc")
            .about("Encode numbers"))
        .subcommand(SubCommand::with_name("dec")
            .about("Decode numbers")
            .arg(Arg::with_name("count")
                .help("count of numbers in encoded input")
                .short("c")
                .long("count")
                .takes_value(true)
                .required(true)))
        .get_matches();

    match matches.subcommand_name() {
        Some("enc") => {
            encode()
        }
        Some("dec") => {
            let count: usize = matches.subcommand_matches("dec").unwrap()
                .value_of("count").unwrap()
                .parse().expect("count must be an int");

            decode(count);
        }
        _ => println!("Invalid subcommand"),
    }
}

fn encode() {
    let stdin = std::io::stdin();
    let stdin_handle = stdin.lock();

    let nums: Vec<u32> = stdin_handle.lines()
        .map(|l| l.expect("Should be able to read stdin"))
        .map(|s| s.parse().expect("Each line must be a u32"))
        .collect();

    let mut encoded = Vec::new();
    encoded.resize(nums.len() * 5, 0);
    let encoded_len = stream_vbyte::encode::<stream_vbyte::Scalar>(&nums, &mut encoded);

    let stdout = std::io::stdout();
    let mut stdout_handle = stdout.lock();
    stdout_handle.write_all(&encoded[0..encoded_len]).expect("Should be able to write to stdout");

    eprintln!("Encoded {} numbers", nums.len());
}

fn decode(count: usize) {
    let stdin = std::io::stdin();
    let mut stdin_handle = stdin.lock();

    let mut encoded = Vec::new();
    stdin_handle.read_to_end(&mut encoded).expect("Should be able to read stdin");

    let mut decoded = Vec::new();
    decoded.resize(count, 0);
    stream_vbyte::decode::<stream_vbyte::Scalar>(&encoded, count, &mut decoded);

    for d in &decoded {
        println!("{}", d);
    }

    eprintln!("Decoded {} numbers", decoded.len());
}
