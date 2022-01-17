use crate::decisions;
use crate::decisions::TestGenDB;
use crate::module_reps::*; // all the representation structs
use crate::test_bodies::*;

use serde_json::Value;
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

    for (func_name, func_desc) in mod_rep.get_mut_fns() {
        let mut cur_cb_position = 0;
        for test_num in 0..decisions::DISCOVERY_PHASE_TESTING_BUDGET {
            let cur_test_file = "js_tools/test.js";
            let args = gen_args_for_fct_with_cb(func_desc, cur_cb_position, testgen_db);
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
            let output = match Command::new("node").arg(cur_test_file).output() {
                Ok(output) => output,
                _ => return Err(DFError::TestRunningError), // should never crash, everything is in a try-catch
            };
            // TODO use the output_json information
            let output_json: Value =
                match serde_json::from_str(match std::str::from_utf8(&output.stdout) {
                    Ok(output_str) => output_str,
                    _ => return Err(DFError::TestOutputParseError),
                }) {
                    Ok(output_json) => output_json,
                    _ => return Err(DFError::TestOutputParseError),
                };
            println!("{:?}", output_json);
        }
    }

    Ok(())
}

// TODO
// right now just the default: takes one arg and it's a callback
fn gen_args_for_fct_with_cb(
    mod_fct: &ModuleFunction,
    cb_position: i32,
    testgen_db: &mut TestGenDB,
) -> Vec<FunctionArgument> {
    let num_args = mod_fct.get_num_api_args();
    let sigs = mod_fct.get_sigs();

    let mut cur_sig = decisions::gen_new_sig_with_cb(num_args, sigs, cb_position, testgen_db);
    print!("{:?}", &cur_sig);
    for arg in cur_sig.get_mut_args() {
        let arg_type = arg.get_type();
        arg.set_string_rep_arg_val(match arg_type {
            ArgType::CallbackType => "cb".to_string(),
            _ => testgen_db.gen_random_value_of_type(arg_type),
        });
    }
    cur_sig.get_arg_list().to_vec()
    // println!("{:?}", cur_sig);
    // TODO get args according to the number of args, and the currently existing signatures
    // args passed in: mod_fct (that has the num_args and the current signatures)
    // and, the callback
    // vec![FunctionArgument::new(
    //     ArgType::CallbackType,
    //     true, // is callback
    //     Some("cb".to_string()),
    // )]
}
