//! API discovery phase of the test generator.
//! These are throwaway tests, not part of the generated suite.
//! Note: this belongs to the old version of the test generator

use crate::consts;
use crate::decisions;
use crate::decisions::TestGenDB;
use crate::errors::*;
use crate::functions::*;
use crate::module_reps::*;
use crate::tests::*;
use crate::TestGenMode;

use std::collections::HashMap;
use std::convert::TryFrom;

/// For all the functions in the module `mod_rep`, run the discovery:
/// We're discovering the position and asynchronicity of callback arguments.
/// The generated (instrumented) tests output a JSON of information about the dynamic
/// types of arguments and returns, which is parsed and then analyzed
/// to construct signatures for the module functions.
pub fn run_discovery_phase(
    mod_rep: NpmModule,
    testgen_db: TestGenDB,
    test_gen_mode: &TestGenMode,
) -> Result<(NpmModule, TestGenDB), DFError> {
    let mut mod_rep = mod_rep;
    let mut testgen_db = testgen_db;
    let mut cur_test_id: usize = 0;

    let mut fcts = mod_rep.get_fns().clone();
    // results of test executions
    let mut test_res_pairs: Vec<(
        Test,
        HashMap<ExtensionPointID, (FunctionCallResult, Option<String>)>,
    )> = Vec::new();

    for ((_, func_name), func_desc) in fcts.iter_mut() {
        let mut cur_cb_position = 1;
        for _ in 0..consts::DISCOVERY_PHASE_TESTING_BUDGET {
            let args = gen_args_for_fct_with_cb(
                &func_desc,
                Some(cur_cb_position - 1),
                &testgen_db,
                &mod_rep,
                test_gen_mode,
            )?;
            let fct_call = FunctionCall::new(
                func_name.clone(),
                FunctionSignature::new(&args, None),
                None,
                None,
                None, // no access path specified (none needed for this legacy code)
                None, // default receiver (the module import)
            );

            let (cur_fct_id, mut cur_test) = Test::test_one_call(
                &mod_rep,
                fct_call.clone(),
                true, /* include basic callback */
                cur_test_id,
                testgen_db.test_dir_path.clone(),
                testgen_db.test_file_prefix.clone(),
                testgen_db.api_src_dir.clone(),
            );

            let test_results = match cur_test.execute() {
                Ok(res) => res.0, // we only care about the hashmap of extension point results in this legacy code (i.e. not APs)
                Err(_) => continue,
            };
            cur_test.delete_file()?;

            let (fct_result, _cb_arg_pos) = test_results.get(&cur_fct_id).unwrap();
            // if there was no execution error, then the generated signature is valid
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

/// Generate arguments for a function with a callback at specified position `cb_position`.
/// If the position specified is invalid (i.e., if it's not in the range of valid indices)
/// then there is no callback argument included.
fn gen_args_for_fct_with_cb(
    mod_fct: &ModuleFunction,
    cb_position: Option<i32>,
    testgen_db: &TestGenDB,
    mod_rep: &NpmModule,
    test_gen_mode: &TestGenMode,
) -> Result<Vec<FunctionArgument>, TestGenError> {
    let num_args = mod_fct.get_num_api_args();
    // TODO in the improved version of the discovery phase, this information will be used
    // to inform the new signatures generated
    let sigs = mod_fct
        .get_sigs()
        .iter()
        .map(|sig| (sig.get_abstract_sig(), 1.0))
        .collect::<HashMap<Vec<ArgType>, f64>>();

    let mut cur_sig =
        decisions::gen_new_sig_with_cb(num_args, &sigs, cb_position, testgen_db, test_gen_mode);
    for (i, arg) in cur_sig.get_mut_args().iter_mut().enumerate() {
        let arg_type = arg.get_type();
        arg.set_arg_val(match arg_type {
            ArgType::CallbackType => ArgVal::Callback(CallbackVal::Var("cb".to_string())),
            _ => testgen_db.gen_random_value_of_type(
                arg_type,
                Some(i),
                &Vec::new(),
                &Vec::new(),
                &mod_rep,
                test_gen_mode,
            ),
        })?;
    }
    Ok(cur_sig.get_arg_list().to_vec())
}
