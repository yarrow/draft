extern crate structopt;
#[macro_use]
extern crate structopt_derive;
use structopt::StructOpt;

use std::io::Read;
use std::path::PathBuf;

extern crate draft;
use draft::CodeFilter;
use draft::Draft;

#[derive(StructOpt, Debug)]
#[structopt(name = "draft")]
/// Extract Rust from Markdown files
struct Opt {
    /// Markdown input file(s)
    #[structopt(parse(from_os_str), required)]
    inputs: Vec<PathBuf>,

    /// Print debugging information
    #[structopt(long = "debug", short = "d")]
    debug: bool,
}

fn print_events(text: &str) {
    for event in CodeFilter::new(text) {
        println!("{:?}", event);
    }
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

#[macro_use]
extern crate failure;
use failure::Error;

fn run() -> Result<(), Error> {
    let opts = Opt::from_args();
    let path = &opts.inputs[0];
    let markdown = slurp(path)?;
    if opts.debug {
        print_events(&markdown);
        return Ok(());
    }

    let web = Draft::new(&markdown);
    if let Some(rust) = web.text_of("") {
        print!("{}", rust);
    } else {
        Err(format_err!("no Rust code found in {:?}", path))?
    }

    Ok(())
}

use std::fs::File;

fn slurp(path: &PathBuf) -> Result<String, Error> {
    let mut result = String::new();
    File::open(path)?.read_to_string(&mut result)?;
    Ok(result)
}
