use crate::decisions::TestGenDB;
use crate::errors::*;
use crate::module_reps::*; // the representation structs, for components
use crate::tests::*;

use rand::Rng;
use std::convert::TryInto;

pub fn run_testgen_phase<'cxt>(
    mod_rep: &'cxt mut NpmModule,
    testgen_db: &'cxt mut TestGenDB,
    num_tests: i32,
) -> Result<(), DFError> {
    // if we specify a nested extension but there's no valid test that can be extended
    // in a nested way, don't error, instead just return a fresh test
    const FRESH_TEST_IF_CANT_EXTEND: bool = true;

    for cur_test_id in 1..=num_tests.try_into().unwrap() {
        let ext_type: ExtensionType = rand::thread_rng().gen();

        let (cur_fct_id, mut cur_test) = Test::extend(
            mod_rep,
            testgen_db,
            ext_type,
            cur_test_id,
            FRESH_TEST_IF_CANT_EXTEND,
        )?;

        let test_results = cur_test.execute()?;
        // TODO WHEN NOT DEBUGGING then, re-print the test not instrumented
        // cur_test.write_test_to_file(false)?;

        testgen_db.set_cur_test_index(cur_test_id);
        testgen_db.add_extension_points_for_test(&cur_test, &test_results);
        println!("Test: {:?} of {:?}", cur_test_id, num_tests);
    }
    Ok(())
}
