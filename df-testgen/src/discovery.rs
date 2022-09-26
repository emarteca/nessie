use crate::decisions;
use crate::decisions::TestGenDB;
use crate::module_reps::*; // all the representation structs
use crate::test_bodies::*;
use crate::testgen::*;

use serde_json::{json, Value};
use std::collections::HashMap;
use std::convert::TryFrom;

/// for all the functions in the mod_rep, run discovery
/// generate tests, and they are instrumented
/// the instrumented tests output a JSON of information about the dynamic types of args and return
/// the output of execution is a JSON, which is parsed and then analyzed
/// then this information is used to construct signatures for the module functions
pub fn run_discovery_phase<'cxt>(
    mod_rep: &'cxt mut NpmModule,
    testgen_db: &'cxt mut TestGenDB<'cxt>,
) -> Result<HashMap<String, ModuleFunction>, DFError> {
    let setup_code = mod_rep.get_js_for_basic_cjs_import();
    let test_header = get_instrumented_header();
    let test_footer = get_instrumented_footer();
    let base_var_name = mod_rep.get_mod_js_var_name();
    let mut cur_test_id = 0;

    let mut fcts = mod_rep.get_fns().clone();

    for (func_name, mut func_desc) in fcts.iter_mut() {
        let mut cur_cb_position = 1;
        for _ in 0..decisions::DISCOVERY_PHASE_TESTING_BUDGET {
            let args = gen_args_for_fct_with_cb(&func_desc, Some(cur_cb_position - 1), testgen_db);
            let fct_call = FunctionCall::new(
                func_name.clone(),
                FunctionSignature::new(args.len(), &args, None),
            );

            let (cur_fct_id, mut cur_test) = testgen_db.build_test_with_call(
                mod_rep,
                fct_call.clone(),
                /* include the basic callback when generating code */ true,
            );
            let test_results = cur_test.execute()?;

            let fct_result = test_results.get(&cur_fct_id).unwrap();
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
        }
    }
    Ok(fcts)
}

/// generate arguments for a function with a callback at specified position
/// if the position specified is invalid (i.e., if it's not in the range of valid indices)
/// then there is no callback argument included
fn gen_args_for_fct_with_cb(
    mod_fct: &ModuleFunction,
    cb_position: Option<i32>,
    testgen_db: &mut TestGenDB,
) -> Vec<FunctionArgument> {
    let num_args = mod_fct.get_num_api_args();
    // TODO in the improved version of the discovery phase, this information will be used
    // to inform the new signatures generated
    let sigs = mod_fct.get_sigs();

    let mut cur_sig = decisions::gen_new_sig_with_cb(num_args, sigs, cb_position, testgen_db);
    for arg in cur_sig.get_mut_args() {
        let arg_type = arg.get_type();
        arg.set_string_rep_arg_val(match arg_type {
            ArgType::CallbackType => "cb".to_string(),
            _ => testgen_db.gen_random_value_of_type(arg_type),
        });
    }
    cur_sig.get_arg_list().to_vec()
}
