use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use structopt::StructOpt;

use nessie::consts;
use nessie::decisions;
use nessie::discovery::run_discovery_phase;
use nessie::mined_seed_reps::MinedNestingPairJSON;
use nessie::module_reps::*; // all the representation structs
use nessie::testgen::run_testgen_phase;

#[derive(Debug, StructOpt)]
#[structopt(name = "nessie_args", about = "Arguments for the test generator")]
struct Opt {
    /// Name of the library/module to generate tests for.
    #[structopt(long)]
    lib_name: String,

    /// Directory containing source code for the library.
    /// Note: this needs to be the root such that if we `require(lib_src_dir)` we
    /// get the library.
    #[structopt(long, parse(from_os_str))]
    lib_src_dir: Option<PathBuf>,

    /// Directory to generate tests into;
    /// if not specified, generate into the current directory.
    #[structopt(long, parse(from_os_str))]
    testing_dir: Option<PathBuf>,

    /// File containing custom import code for the library.
    /// If this is not specified, then we use the default `require(lib-name or lib-src-dir)`.
    #[structopt(long, parse(from_os_str))]
    module_import_code: Option<PathBuf>,

    /// Number of tests to generate.
    #[structopt(long)]
    num_tests: i32,

    /// Running the discovery phase?
    /// Default: no if there is an existing discovery output file.
    #[structopt(long)]
    run_discover: bool,

    /// Running the test generation phase?
    /// Default: yes.
    #[structopt(long)]
    skip_testgen: bool,

    /// File containing mined data.
    #[structopt(long, parse(from_os_str))]
    mined_data: Option<PathBuf>,
}

/// Function to set up a toy filesystem that the generated tests can interact with.
fn setup_toy_fs(path_start: &str) -> Result<Vec<PathBuf>, std::io::Error> {
    let mut toy_fs_paths: Vec<PathBuf> = Vec::new();

    for dir in &consts::setup::TOY_FS_DIRS {
        let cur_path = PathBuf::from(path_start.to_owned() + "/" + dir);
        toy_fs_paths.push(cur_path.clone());
        if Path::new(&(cur_path)).exists() {
            continue;
        }
        std::fs::DirBuilder::new()
            .recursive(true)
            .create(&cur_path)?;
    }

    for file in &consts::setup::TOY_FS_FILES {
        let cur_path = PathBuf::from(path_start.to_owned() + "/" + file);
        toy_fs_paths.push(cur_path.clone());
        if Path::new(&(cur_path)).is_file() {
            continue;
        }
        std::fs::File::create(cur_path)?;
    }

    Ok(toy_fs_paths)
}

fn main() {
    let opt = Opt::from_args();

    let discovery_filename = "js_tools/".to_owned() + &opt.lib_name + "_discovery.json";

    let testing_dir = match &opt.testing_dir {
        Some(ref dir) => dir.clone().into_os_string().into_string().unwrap(),
        None => String::from("."),
    };

    let test_dir_path = consts::setup::TEST_DIR_PATH;

    let toy_dir_base = &(testing_dir.clone() + "/" + test_dir_path + "/toy_fs_dir");
    let toy_fs_paths =
        setup_toy_fs(&toy_dir_base).expect("Error creating toy filesystem for tests; bailing out.");

    let mined_data: Option<Vec<MinedNestingPairJSON>> = if let Some(ref mined_data_file) =
        opt.mined_data
    {
        Some(
            MinedNestingPairJSON::list_from_file(mined_data_file)
                .expect(format!("failed to read mined data from {:?}", opt.mined_data).as_str()),
        )
    } else {
        None
    };

    let test_file_prefix = consts::setup::TEST_FILE_PREFIX;

    // setup the initial test gen database.
    let mut testgen_db = decisions::TestGenDB::new(
        test_dir_path.to_string(),
        test_file_prefix.to_string(),
        mined_data,
        match &opt.lib_src_dir {
            Some(ref dir) => Some(
                std::fs::canonicalize(dir.clone())
                    .expect(format!("invalid directory {:?} for api source code", dir).as_str())
                    .into_os_string()
                    .into_string()
                    .unwrap(),
            ),
            None => None,
        },
    );
    testgen_db.set_fs_strings(toy_fs_paths, toy_dir_base);

    // if we don't have the source code of the api, install it so it can be `require`d
    if !opt.lib_src_dir.is_some() {
        if !Path::new(&("node_modules/".to_owned() + &opt.lib_name)).exists() {
            Command::new("npm")
                .arg("install")
                .arg(&opt.lib_name)
                .output()
                .expect(format!("failed to install {:?} to test", &opt.lib_name).as_str());
        }
    }

    // if discovery file doesn't already exist
    let (mut mod_rep, mut testgen_db) =
        if (!Path::new(&discovery_filename).exists()) || opt.run_discover {
            // is the api spec file already there? if so, don't run
            let api_spec_filename = "js_tools/".to_owned() + &opt.lib_name + "_output.json";
            let mut api_spec_args = vec!["lib_name=".to_owned() + &opt.lib_name.clone()];
            if let Some(ref dir) = opt.lib_src_dir {
                let lib_src_dir_name = dir.clone().into_os_string().into_string().unwrap();
                api_spec_args.push("lib_src_dir=".to_owned() + &lib_src_dir_name);
            }
            if let Some(ref import_file) = opt.module_import_code {
                let import_file_name = import_file.clone().into_os_string().into_string().unwrap();
                api_spec_args.push("import_code_file=".to_owned() + &import_file_name);
            }

            if !Path::new(&api_spec_filename).exists() {
                Command::new("./get_api_specs.sh")
                    .args(api_spec_args)
                    .output()
                    .expect(
                        format!(
                            "failed to execute API info gathering process for {:?}",
                            &opt.lib_name
                        )
                        .as_str(),
                    );
                println!("Generating API spec");
            } else {
                println!(
                    "API spec file exists, reading from {:?}",
                    &api_spec_filename
                );
            }

            // if we got to this point, we successfully got the API and can construct the module object
            let mod_rep = match NpmModule::from_api_spec(
                PathBuf::from(&api_spec_filename),
                opt.lib_name.clone(),
                opt.module_import_code,
            ) {
                Ok(mod_rep) => mod_rep,
                _ => panic!("Error reading the module spec from the api_info file"),
            };
            (mod_rep, testgen_db)
        } else {
            let file_conts_string = std::fs::read_to_string(&discovery_filename).unwrap();
            (
                serde_json::from_str(&file_conts_string).unwrap(),
                testgen_db,
            )
        };

    // at this point, the mod_rep has the results from the API listing phase, or
    // a previously run discovery if applicable

    let num_tests = opt.num_tests;
    if !opt.skip_testgen {
        run_testgen_phase(&mut mod_rep, &mut testgen_db, num_tests)
            .expect("Error running test generation phase: {:?}");
    } else {
        println!("`skip-testgen` specified: Skipping test generation phase.")
    }

    let mut disc_file =
        std::fs::File::create(&discovery_filename).expect("Error creating discovery JSON file");
    // print discovery to a file
    disc_file
        .write_all(format!("{:?}", mod_rep).as_bytes())
        .expect("Error writing to discovery JSON file AFTER TESTGEN OMG");
}
