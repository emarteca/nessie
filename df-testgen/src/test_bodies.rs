use crate::module_reps::*; // all the representation structs
use crate::testgen::FunctionCall;

/// simplest callback: just print that it has executed
pub fn basic_callback() -> &'static str {
    r#"let cb = function() { console.log({"callback_exec": true}); }"#
}

pub fn basic_callback_with_id(cur_call_uniq_id: String) -> String {
    "let cb = function() { console.log({\"callback_exec_".to_owned()
        + &cur_call_uniq_id
        + "\": true}); }"
}

/// returns a string of JS code that redefines the console.log
/// printing function so that it pushes the argument to console.log
/// onto an array.
/// this instrumentation allows us to track what's being printed and
/// in what order
pub fn get_instrumented_header() -> &'static str {
    r#"
let orig_log = console.log;
let output_log = [];
console.log = function(e) {
	output_log.push(e);
}"#
}

/// returns a string of JS code that prints the global array that
/// console.log is redefined to add to, to the console on process exit
/// (if there are async functions, this will be after all the async functions
/// have finished executing)
pub fn get_instrumented_footer() -> &'static str {
    r#"
process.on("exit", function f() {
	orig_log(JSON.stringify(output_log));
})"#
}

/// wrap the given call in a try-catch
/// print the return value, and the argument values before the call
/// print an error in the catch
/// remember at this point "print" has been redefined to push to the array
pub fn get_instrumented_function_call(
    cur_node_call_sig: &FunctionSignature,
    fct_name: &str,
    args_rep: String,
    ret_val_basename: String,
    extra_cb_code: String,
    base_var_name: &str,
    cur_call_uniq_id: String,
    indents: String,
) -> String {
    let print_args = |title: String| {
        if cur_node_call_sig.is_spread_args {
            [
                "\tconsole.log({\"",
                &title,
                "_",
                &cur_call_uniq_id,
                "_",
                &ret_val_basename,
                "_args\": args});",
            ]
            .join("")
        } else {
            let args = cur_node_call_sig.get_arg_list();
            args.iter()
                .enumerate()
                .map(|(i, fct_arg)| {
                    [
                        "\tconsole.log({\"",
                        &title,
                        "_",
                        &cur_call_uniq_id,
                        "_",
                        &ret_val_basename,
                        "_arg",
                        &i.to_string(),
                        "\": ",
                        &fct_arg
                            .get_string_rep_arg_val__short()
                            .as_ref()
                            .unwrap()
                            .clone(),
                        "});",
                    ]
                    .join("")
                })
                .collect::<Vec<String>>()
                .join("\n")
        }
    };
    [
        "\ntry { ",
        &extra_cb_code,
        &print_args("before_cb".to_string()),
        &("\tlet ".to_owned()
            + &ret_val_basename
            + " = "
            + base_var_name
            + "."
            + &fct_name
            + "("
            + &args_rep
            + ");"),
        &print_args("after_cb".to_string()),
        &("\tconsole.log({\"".to_owned()
            + &ret_val_basename
            + "_"
            + &cur_call_uniq_id
            + "\": typeof "
            + &ret_val_basename
            + " == \"function\"? \"[function]\" : "
            + &ret_val_basename
            + ".toString()});"),
        &("\tconsole.log({\"ret_val_type_".to_owned()
            + &cur_call_uniq_id
            + "\": typeof "
            + &ret_val_basename
            + "});"),
        // rejected promise
        &("\tPromise.resolve(".to_owned()
            + &ret_val_basename
            + ").catch(e => { console.log({\"error_"
            + &cur_call_uniq_id
            + "\": true}); });"),
        "} catch(e) {",
        &("\tconsole.log({\"error_".to_owned() + &cur_call_uniq_id + "\": true});"),
        "}",
        &("console.log({\"done_".to_owned() + &cur_call_uniq_id + "\": true});"),
    ]
    .join("\n")
    .replace("\n", &("\n".to_owned() + &indents))
}
