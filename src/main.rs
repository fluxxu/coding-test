use std::io::BufReader;
use std::path::PathBuf;
use std::fs::File;

mod engine;
mod error;

use crate::engine::{Engine, EngineTransaction, CsvReader};
use crate::error::Error;

use clap::Parser;

/// A toy transaction processing engine
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to the CSV file to process
    path: PathBuf,

    /// Verbose mode
    #[arg(short, long, default_value_t = false)]
    verbose: bool,
}

fn main() -> Result<(), Error> {
    let args = Args::parse();

    let r = File::open(args.path)?;
    let r = BufReader::new(r);
    let mut csv_reader = CsvReader::from_reader(r)?;
    let mut engine = Engine::new();

    let mut line_number = 1;
    while let Some(record) = csv_reader.read_next()? {
        line_number += 1;
        let tx = EngineTransaction::parse_csv_record(&record)?;
        if let Err(e) = engine.process_transaction(tx) {
            if args.verbose {
                eprintln!(
                    "Transaction rejected at line {}: type: {:?}, client: {}, tx: {}, error: {}",
                    line_number, record.r#type, record.client, record.tx, e
                );
            }
        }
    }

    let mut w = ::csv::Writer::from_writer(std::io::stdout());

    for item in engine.output_items() {
        w.serialize(item).map_err(Error::WriteCsvRecord)?;
    }
    w.flush()?;
    Ok(())
}
