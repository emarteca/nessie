use crate::decisions;
use crate::decisions::TestGenDB;
use crate::module_reps::*; // all the representation structs
use crate::test_bodies::*;

use serde_json::{json, Value};
use std::convert::TryFrom;
use std::process::Command;

/// simplest callback: just print that it has executed
pub fn basic_callback() -> &'static str {
    r#"let cb = function() { console.log({"callback_exec": true}); }"#
}

/// for all the functions in the mod_rep, run discovery
/// generate tests, and they are instrumented
/// the instrumented tests output a JSON of information about the dynamic types of args and return
/// the output of execution is a JSON, which is parsed and then analyzed
/// then this information is used to construct signatures for the module functions
pub fn run_discovery_phase(
    mod_rep: &mut NpmModule,
    testgen_db: &mut TestGenDB,
) -> Result<(), DFError> {
    let setup_code = mod_rep.get_js_for_basic_cjs_import();
    let test_header = get_instrumented_header();
    let test_footer = get_instrumented_footer();
    let base_var_name = mod_rep.get_mod_js_var_name();
    let mut cur_test_id = 0;

    for (func_name, func_desc) in mod_rep.get_mut_fns() {
        let mut cur_cb_position = 1;
        for _ in 0..decisions::DISCOVERY_PHASE_TESTING_BUDGET {
            let cur_test_file = "js_tools/test".to_owned() + &cur_test_id.to_string() + ".js";
            let args = gen_args_for_fct_with_cb(func_desc, cur_cb_position - 1, testgen_db);
            let test_call = get_instrumented_function_call(func_name, &base_var_name, &args);

            let cur_test = [
                test_header,
                &setup_code,
                basic_callback(),
                &test_call,
                test_footer,
            ]
            .join("\n");
            if matches!(std::fs::write(&cur_test_file, cur_test), Err(_)) {
                return Err(DFError::WritingTestError);
            }
            let output = match Command::new("node").arg(&cur_test_file).output() {
                Ok(output) => output,
                _ => return Err(DFError::TestRunningError), // should never crash, everything is in a try-catch
            };
            let output_json: Value = 
                match serde_json::from_str(match std::str::from_utf8(&output.stdout) {
                    Ok(output_str) => output_str,
                    _ => return Err(DFError::TestOutputParseError),
                }) {
                    Ok(output_json) => output_json,
                    _ => return Err(DFError::TestOutputParseError),
                };
            // if the test didn't error, then we found a valid signature
            let test_result = diagnose_single_callback_correctness(&output_json);
            if test_result != SingleCallCallbackTestResult::ExecutionError {
                func_desc.add_sig(FunctionSignature::try_from((&args, test_result)).unwrap());
                print!("{:?}", cur_test_file);
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
    Ok(())
}

/// look at the JSON output of running a test with a single call, and determine what that means
/// for the argument list. This focuses on callback execution: is the callback executed, and
/// in what order relative to the execution of the main thread of the test? Also, did the call error?
fn diagnose_single_callback_correctness(output_json: &Value) -> SingleCallCallbackTestResult {
    let output_vec = match output_json {
        Value::Array(vec) => vec,
        _ => return SingleCallCallbackTestResult::ExecutionError,
    };
    if matches!(
        output_vec.iter().position(|r| r == &json!({"error": true})),
        Some(_)
    ) {
        return SingleCallCallbackTestResult::ExecutionError;
    }
    // now look through and see if the callback was executed
    // and if so, whether or not it was executed sequentially
    let done_pos = output_vec.iter().position(|r| r == &json!({"done": true}));
    let callback_pos = output_vec
        .iter()
        .position(|r| r == &json!({"callback_exec": true}));

    match (done_pos, callback_pos) {
        (Some(done_index), Some(callback_index)) => {
            // if test ends before callback is done executing, it's async
            if done_index < callback_index {
                SingleCallCallbackTestResult::CallbackCalledAsync
            }
            // else it's sync
            else {
                SingleCallCallbackTestResult::CallbackCalledSync
            }
        }
        (Some(_), None) => SingleCallCallbackTestResult::NoCallback,
        // if "done" never prints, there was an error
        _ => SingleCallCallbackTestResult::ExecutionError,
    }
}

/// generate arguments for a function with a callback at specified position
/// if the position specified is invalid (i.e., if it's not in the range of valid indices)
/// then there is no callback argument included
fn gen_args_for_fct_with_cb(
    mod_fct: &ModuleFunction,
    cb_position: i32,
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
