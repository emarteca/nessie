use std::path::{Path, PathBuf};
use std::process::Command;
use structopt::StructOpt;

use df_testgen::module_reps::*; // all the representation structs

use df_testgen::decisions;
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

/// function to set up a toy filesystem that the generated tests can interact with
fn setup_toy_fs(path_start: &str) -> Result<Vec<PathBuf>, std::io::Error> {
    let mut toy_fs_paths: Vec<PathBuf> = Vec::new();

    for dir in &decisions::SETUP::TOY_FS_DIRS {
        let cur_path = PathBuf::from(path_start.to_owned() + "/" + dir);
        std::fs::DirBuilder::new()
            .recursive(true)
            .create(&cur_path)?;
        toy_fs_paths.push(cur_path);
    }

    for file in &decisions::SETUP::TOY_FS_FILES {
        let cur_path = PathBuf::from(path_start.to_owned() + "/" + file);
        std::fs::File::create(path_start.to_owned() + "/" + file)?;
        toy_fs_paths.push(cur_path);
    }

    Ok(toy_fs_paths)
}

fn main() {
    let opt = Opt::from_args();

    // is the api spec file already there? if so, don't run
    let api_spec_filename = "js_tools/".to_owned() + &opt.lib_name + "_output.json";

    if !Path::new(&api_spec_filename).exists() {
        Command::new("./get_api_specs.sh")
            .arg(&opt.lib_name)
            .output()
            .expect(
                format!(
                    "failed to execute API info gathering process for {:?}",
                    &opt.lib_name
                )
                .as_str(),
            );
    }

    // if we got to this point, we successfully got the API and can construct the module object
    let mut mod_rep =
        match NpmModule::from_api_spec(PathBuf::from(&api_spec_filename), opt.lib_name) {
            Ok(mod_rep) => mod_rep,
            _ => panic!("Error reading the module spec from the api_info file"),
        };

    let toy_fs_paths = setup_toy_fs("js_tools/toy_fs_dir")
        .expect("Error creating toy filesystem for tests; bailing out.");

    let mut testgen_db = decisions::TestGenDB::new();
    testgen_db.set_fs_strings(toy_fs_paths);

    if let Err(e) = run_discovery_phase(&mut mod_rep, &mut testgen_db) {
        panic!("Error running discovery phase: {:?}", e);
    }

    let _num_tests = opt.num_tests;
}
