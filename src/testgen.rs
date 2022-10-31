use crate::code_gen;
use crate::consts;
use crate::decisions::TestGenDB;
use crate::errors::*;
use crate::module_reps::*; // the representation structs, for components
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
    for cur_test_id in 1..=num_tests.try_into().unwrap() {
        // get a random extension type
        let ext_type: ExtensionType = rand::thread_rng().gen();

        let (cur_fct_id, mut cur_test) = Test::extend(
            mod_rep,
            testgen_db,
            ext_type,
            cur_test_id,
            consts::FRESH_TEST_IF_CANT_EXTEND,
        )?;

        let test_results = cur_test.execute()?;

        // after running the test, reprint file without all the instrumentation
        // and as part of a mocha test suite
        cur_test.write_test_to_file(
            false, /* no instrumentation */
            true,  /* as part of a mocha test suite */
        )?;

        testgen_db.set_cur_test_index(cur_test_id);
        testgen_db.add_extension_points_for_test(&cur_test, &test_results);
        println!("Test: {:?} of {:?}", cur_test_id, num_tests);
    }
    // print the runner for the mocha test suite
    write_meta_test(testgen_db.test_dir_path.clone(), num_tests);
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
