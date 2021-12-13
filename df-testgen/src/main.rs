use std::{fs, io, path::PathBuf};
use structopt::StructOpt;
use std::process::Command;

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
}

fn main() {

    let opt = Opt::from_args();

    let output = Command::new("./get_api_specs.sh")
                     .arg(&opt.lib_name)
                     .output()
                     .expect(format!("failed to execute API info gathering process for {:?}", &opt.lib_name).as_str());

    println!("{:?}", opt);
}
