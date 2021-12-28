use std::process::Command;
use std::{path::PathBuf};
use structopt::StructOpt;


use df_testgen::module_reps::*; // all the representation structs

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

    let _output = Command::new("./get_api_specs.sh")
        .arg(&opt.lib_name)
        .output()
        .expect(
            format!(
                "failed to execute API info gathering process for {:?}",
                &opt.lib_name
            )
            .as_str(),
        );

    let api_spec_filename = "js_tools/".to_owned() + &opt.lib_name + "_output.json";

    // if we got to this point, we successfully got the API and can construct the module object
    let mod_rep = match NpmModule::from_api_spec(PathBuf::from(api_spec_filename), opt.lib_name) {
        Ok(mod_rep) => mod_rep,
        _ => panic!("Error reading the module spec from the api_info file")
    };
    print!("{:?}", mod_rep);

    let _num_tests = opt.num_tests;
}
