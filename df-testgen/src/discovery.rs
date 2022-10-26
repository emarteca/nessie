use crate::decisions;
use crate::decisions::TestGenDB;
use crate::module_reps::*; // all the representation structs
use crate::testgen::*;

use std::collections::HashMap;
use std::convert::TryFrom;

/// for all the functions in the mod_rep, run discovery
/// generate tests, and they are instrumented
/// the instrumented tests output a JSON of information about the dynamic types of args and return
/// the output of execution is a JSON, which is parsed and then analyzed
/// then this information is used to construct signatures for the module functions
pub fn run_discovery_phase(
    mod_rep: NpmModule,
    testgen_db: TestGenDB,
) -> Result<(NpmModule, TestGenDB), DFError> {
    let mut mod_rep = mod_rep;
    let mut testgen_db = testgen_db;
    let mut cur_test_id: usize = 0;

    let mut fcts = mod_rep.get_fns().clone();
    let mut test_res_pairs: Vec<(
        Test,
        HashMap<ExtensionPointID, (FunctionCallResult, Option<String>)>,
    )> = Vec::new();

    for (func_name, func_desc) in fcts.iter_mut() {
        let mut cur_cb_position = 1;
        for _ in 0..decisions::DISCOVERY_PHASE_TESTING_BUDGET {
            let args =
                gen_args_for_fct_with_cb(&func_desc, Some(cur_cb_position - 1), &testgen_db)?;
            let fct_call = FunctionCall::new(
                func_name.clone(),
                FunctionSignature::new(&args, None),
                None,
                None,
            );

            let (cur_fct_id, mut cur_test) = Test::test_one_call(
                &mod_rep,
                fct_call.clone(),
                true, /* include basic callback */
                cur_test_id,
                testgen_db.get_test_dir_path(),
                testgen_db.get_test_file_prefix(),
            );

            let test_results = cur_test.execute()?;

            let (fct_result, cb_arg_pos) = test_results.get(&cur_fct_id).unwrap();
            if fct_result != &FunctionCallResult::ExecutionError {
                func_desc.add_sig(FunctionSignature::try_from((&args, *fct_result)).unwrap());
            }

            // if we haven't tested the current position with no callbacks, do that
            // else, move to the next position in the arg list and try with a callback arg
            if cur_cb_position < 0 && args.len() > 0 {
                cur_cb_position =
                    (((cur_cb_position * (-1)) + 1) % i32::try_from(args.len()).unwrap()) + 1
            } else {
                cur_cb_position *= -1
            }
            cur_test_id += 1;
            test_res_pairs.push((cur_test, test_results));
        }
    }
    testgen_db.set_cur_test_index(cur_test_id);
    for (cur_test, test_results) in test_res_pairs.iter() {
        testgen_db.add_extension_points_for_test(cur_test, test_results);
    }
    mod_rep.set_fns(fcts);
    Ok((mod_rep, testgen_db))
}

/// generate arguments for a function with a callback at specified position
/// if the position specified is invalid (i.e., if it's not in the range of valid indices)
/// then there is no callback argument included
fn gen_args_for_fct_with_cb(
    mod_fct: &ModuleFunction,
    cb_position: Option<i32>,
    testgen_db: &TestGenDB,
) -> Result<Vec<FunctionArgument>, TestGenError> {
    let num_args = mod_fct.get_num_api_args();
    // TODO in the improved version of the discovery phase, this information will be used
    // to inform the new signatures generated
    let sigs = mod_fct.get_sigs();

    let mut cur_sig = decisions::gen_new_sig_with_cb(num_args, sigs, cb_position, testgen_db);
    for (i, arg) in cur_sig.get_mut_args().iter_mut().enumerate() {
        let arg_type = arg.get_type();
        arg.set_arg_val(match arg_type {
            ArgType::CallbackType => ArgVal::Callback(CallbackVal::Var("cb".to_string())),
            _ => testgen_db.gen_random_value_of_type(arg_type, Some(i), &Vec::new(), &Vec::new()),
        })?;
    }
    Ok(cur_sig.get_arg_list().to_vec())
}
