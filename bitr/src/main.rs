mod btor2;
mod bitr;
mod blast;
mod oracle;
mod bmc;
mod stats;

use std::env;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: bitr [OPTIONS] <file.btor2>");
        eprintln!("Options:");
        eprintln!("  --verbose    Verbose output");
        eprintln!("  --stats      Print statistics");
        eprintln!("  --timeout N  Timeout in seconds (default: 300)");
        process::exit(1);
    }

    let mut verbose = false;
    let mut _print_stats = false;
    let mut _timeout_s: f64 = 300.0;
    let mut filename = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--verbose" => verbose = true,
            "--stats" => _print_stats = true,
            "--timeout" => {
                i += 1;
                _timeout_s = args[i].parse().unwrap_or(300.0);
            }
            arg if !arg.starts_with('-') => filename = Some(arg.to_string()),
            other => {
                eprintln!("Unknown option: {}", other);
                process::exit(1);
            }
        }
        i += 1;
    }

    let filename = match filename {
        Some(f) => f,
        None => {
            eprintln!("Error: no input file specified");
            process::exit(1);
        }
    };

    if verbose {
        eprintln!("bitr: loading {}", filename);
    }

    // TODO: Parse BTOR2, build BVDDs, run solver
    eprintln!("bitr: not yet implemented");
    process::exit(1);
}
