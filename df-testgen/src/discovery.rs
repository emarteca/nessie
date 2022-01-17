use crate::module_reps::*; // all the representation structs
use crate::test_bodies::*;
use crate::decisions;

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
pub fn run_discovery_phase(mod_rep: &mut NpmModule) -> Result<(), DFError> {
    let setup_code = mod_rep.get_js_for_basic_cjs_import();
    let test_header = get_instrumented_header();
    let test_footer = get_instrumented_footer();
    let base_var_name = mod_rep.get_mod_js_var_name();

    for (func_name, func_desc) in mod_rep.get_mut_fns() {
        for test_num in 0..decisions::DISCOVERY_PHASE_TESTING_BUDGET {
            let cur_test_file = "js_tools/test.js";
            let args = gen_args_for_fct(func_desc);
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
fn gen_args_for_fct(mod_fct: &ModuleFunction) -> Vec<FunctionArgument> {
    let num_args = mod_fct.get_num_api_args();
    let sigs = mod_fct.get_sigs();

    let cur_sig = decisions::gen_new_sig(num_args, sigs);
    // TODO get args according to the number of args, and the currently existing signatures
    // args passed in: mod_fct (that has the num_args and the current signatures)
    // and, the callback
    vec![FunctionArgument::new(
        ArgType::CallbackType,
        true, // is callback
        "cb".to_string(),
    )]
}
