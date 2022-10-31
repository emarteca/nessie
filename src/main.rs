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
    /// name of the library/module to generate tests for
    #[structopt(long)]
    lib_name: String,

    /// file containing source code for the library
    /// note: this needs to be the root such that if we `require(lib_src_dir)` we
    /// get the library
    #[structopt(long, short, parse(from_os_str))]
    lib_src_dir: Option<PathBuf>,

    /// if not specified, generate into the current directory
    #[structopt(long, short, parse(from_os_str))]
    testing_dir: Option<PathBuf>,

    /// number of tests to generate
    #[structopt(long)]
    num_tests: i32,

    /// running the discovery phase? default: no if there is an existing discovery output file
    #[structopt(long)]
    run_discover: bool,

    /// running the test generation phase? default: yes
    #[structopt(long)]
    skip_testgen: bool,

    /// file containing mined data
    #[structopt(long, short, parse(from_os_str))]
    mined_data: Option<PathBuf>,
}

/// function to set up a toy filesystem that the generated tests can interact with
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
        if Path::new(&(cur_path)).exists() {
            continue;
        }
        std::fs::File::create(path_start.to_owned() + "/" + file)?;
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

    let toy_fs_paths = setup_toy_fs(&(testing_dir.clone() + "/" + test_dir_path + "/toy_fs_dir"))
        .expect("Error creating toy filesystem for tests; bailing out.");

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
    testgen_db.set_fs_strings(toy_fs_paths);

    // if discovery file doesn't already exist
    let (mut mod_rep, mut testgen_db) = if (!Path::new(&discovery_filename).exists())
        || opt.run_discover
    {
        // is the api spec file already there? if so, don't run
        let api_spec_filename = "js_tools/".to_owned() + &opt.lib_name + "_output.json";
        let mut api_spec_args = vec![opt.lib_name.clone()];
        if let Some(ref dir) = opt.lib_src_dir {
            let lib_src_dir_name = dir.clone().into_os_string().into_string().unwrap();
            api_spec_args.push(lib_src_dir_name);
        }

        if !Path::new(&api_spec_filename).exists() {
            let mut command = Command::new("./get_api_specs.sh")
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

        // if we got to this point, we successfully got the API and can construct the module object
        let mut mod_rep =
            match NpmModule::from_api_spec(PathBuf::from(&api_spec_filename), opt.lib_name.clone())
            {
                Ok(mod_rep) => mod_rep,
                _ => panic!("Error reading the module spec from the api_info file"),
            };
        let (mod_rep, testgen_db) =
            run_discovery_phase(mod_rep, testgen_db).expect("Error running discovery phase: {:?}");
        let mut disc_file =
            std::fs::File::create(&discovery_filename).expect("Error creating discovery JSON file");
        // print discovery to a file
        disc_file
            .write_all(format!("{:?}", mod_rep).as_bytes())
            .expect("Error writing to discovery JSON file");
        (mod_rep, testgen_db)
    } else {
        let file_conts_string = std::fs::read_to_string(&discovery_filename).unwrap();
        (
            serde_json::from_str(&file_conts_string).unwrap(),
            testgen_db,
        )
    };

    // at this point, the mod_rep has the results from the discovery phase

    println!("Discovery phase returns: {}", mod_rep.short_display());

    let num_tests = opt.num_tests;

    run_testgen_phase(&mut mod_rep, &mut testgen_db, num_tests);
}