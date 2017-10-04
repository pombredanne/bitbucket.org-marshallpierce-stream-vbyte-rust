extern crate stream_vbyte;

use std::io::{BufRead, Read, Write};

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <enc|dec> [count, if mode=dec]", args[0]);
        std::process::exit(1);
    }

    // TODO use clap so arg parsing is less messy

    match args[1].as_str() {
        "enc" => {
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
        "dec" => {
            let count: usize = args[2].parse().expect("Arg to 'dec' must be a number");
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
        _ => {
            eprintln!("Uknown mode");
            std::process::exit(1);
        }
    }
}
