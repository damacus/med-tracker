use std::{env, io, path::Path, process::ExitCode};

use med_tracker::models::clinical_transfer::prepare_files;

const USAGE: &str = "Usage: prepare-clinical-transfer PAYLOAD MANIFEST OUTPUT < passphrase-file\nPrepare a complete household v2 export for Rust v3. Never overwrites OUTPUT.\nPass only the resulting .bundle to the existing import API, with explicit current guardian declarations.\nExit codes: 0 prepared; 1 validation or I/O failure; 2 usage error.";

fn main() -> ExitCode {
    let args: Vec<_> = env::args_os().skip(1).collect();
    if args.len() == 1 && args[0] == "--help" {
        println!("{USAGE}");
        return ExitCode::SUCCESS;
    }
    if args.len() != 3 {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    }
    match prepare_files(
        Path::new(&args[0]),
        Path::new(&args[1]),
        Path::new(&args[2]),
        io::stdin().lock(),
    ) {
        Ok(()) => {
            eprintln!(
                "Prepared encrypted bundle. Review its conversion receipt, then run the existing import API dry-run with current guardian declarations."
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
