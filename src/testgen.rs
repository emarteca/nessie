//! Driver for the generation of the tests.

use crate::code_gen;
use crate::consts;
use crate::decisions::TestGenDB;
use crate::errors::*;
use crate::module_reps::*;
use crate::tests::*;

use rand::Rng;
use std::convert::TryInto;
use std::path::PathBuf;

/// Generate `num_tests` number of tests, for the specified module.
pub fn run_testgen_phase<'cxt>(
    mod_rep: &'cxt mut NpmModule,
    testgen_db: &'cxt mut TestGenDB,
    num_tests: i32,
) -> Result<(), DFError> {
    let mut cur_test_id = 1;
    while cur_test_id <= num_tests.try_into().unwrap() {
        // get a random extension type
        let ext_type: ExtensionType = rand::thread_rng().gen();

        let (_cur_fct_id, mut cur_test) = Test::extend(
            mod_rep,
            testgen_db,
            ext_type,
            cur_test_id,
            consts::FRESH_TEST_IF_CANT_EXTEND,
        )?;

        // if there's an error in a test execution (e.g., timeout), just keep going with the
        // rest of the tests but don't add this test to the valid pool
        // HEURISTIC: don't increment the test ID number. Technically this makes the worst
        // case complexity infinite, but in practice this doesn't happen enough to be a problem.
        // Revisit if this ends up being a problem with other packages.
        let test_results = match cur_test.execute() {
            Ok(res) => res,
            Err(_) => {
                println!(
                    "Execution error in generating test {:?} -- retrying",
                    cur_test_id
                );
                continue;
            }
        };

        // after running the test, reprint file without all the instrumentation
        // and as part of a mocha test suite
        // cur_test.write_test_to_file(
        //     false, /* no instrumentation */
        //     true,  /* as part of a mocha test suite */
        // )?;

        testgen_db.set_cur_test_index(cur_test_id);
        mod_rep.add_fcts_rooted_in_ret_vals(&test_results.1);
        mod_rep.add_function_sigs_from_test(&cur_test, &test_results.0);
        testgen_db.add_extension_points_for_test(&cur_test, &test_results.0);
        println!("Test: {:?} of {:?}", cur_test_id, num_tests);

        cur_test_id = cur_test_id + 1;
    }
    // print the runner for the mocha test suite
    write_meta_test(testgen_db.test_dir_path.clone(), num_tests)?;
    Ok(())
}

/// Print the test suite runner for `num_tests` generated tests.
pub fn write_meta_test(test_dir: String, num_tests: i32) -> Result<(), DFError> {
    let meta_test_code = code_gen::get_meta_test_code(num_tests);
    let meta_test_file = PathBuf::from(test_dir + "/metatest.js");
    if matches!(std::fs::write(&meta_test_file, meta_test_code), Err(_)) {
        return Err(DFError::WritingTestError);
    }
    Ok(())
}
