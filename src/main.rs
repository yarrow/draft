extern crate structopt;
#[macro_use]
extern crate structopt_derive;
use structopt::StructOpt;

use std::io::Read;
use std::path::PathBuf;

extern crate draft;
use draft::tangle::Tangle;

#[derive(StructOpt, Debug)]
#[structopt(name = "draft")]
/// Extract Rust from Markdown files
struct Opt {
    /// Markdown input file(s)
    #[structopt(parse(from_os_str), required)]
    inputs: Vec<PathBuf>,
}

use std::process;
fn main() {
    match run() {
        Ok(_) => (),
        Err(e) => {
            eprintln!("{}", e);
            process::exit(1);
        }
    }
}

extern crate failure;
use failure::Error;

fn run() -> Result<(), Error> {
    let opts = Opt::from_args();
    let path = &opts.inputs[0];
    let markdown = slurp(path)?;

    let tangle = Tangle::new(&markdown);
    print!("{}", tangle.get("")?);
    Ok(())
}

use std::fs::File;

fn slurp(path: &PathBuf) -> Result<String, Error> {
    let mut result = String::new();
    File::open(path)?.read_to_string(&mut result)?;
    Ok(result)
}
