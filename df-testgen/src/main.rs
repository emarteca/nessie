use std::{fs, io, path::PathBuf};
use structopt::StructOpt;
use std::process::Command;

use df_testgen::discovery::run_discovery_phase;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "df_testgen_args",
    about = "Arguments for the DF test generator"
)]
struct Opt {
    /// name of the library/module to generate tests for
    #[structopt(long)]
    lib_name: String,

    /// file containing setup code for the library
    #[structopt(long, short, parse(from_os_str))]
    lib_setup_code: Option<PathBuf>,

    /// number of tests to generate
    #[structopt(long)]
    num_tests: i32,

    /// running the discovery phase? default: no if there is an existing discovery output file
    #[structopt(long)]
    run_discover: bool,

    /// running the test generation phase? default: yes
    #[structopt(long)]
    skip_testgen: bool,
}

fn main() {

    let opt = Opt::from_args();

    let output = Command::new("./get_api_specs.sh")
                     .arg(&opt.lib_name)
                     .output()
                     .expect(format!("failed to execute API info gathering process for {:?}", &opt.lib_name).as_str());

    let num_tests = opt.num_tests;

    println!("{:?}", opt);
}
