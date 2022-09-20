use crate::module_reps::*; // all the representation structs

/// simplest callback: just print that it has executed
pub fn basic_callback() -> &'static str {
    r#"let cb = function() { console.log({"callback_exec": true}); }"#
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
    fct_name: &str,
    base_var_name: &str,
    args: &Vec<FunctionArgument>,
) -> String {
    let ret_val_basename = "ret_val_".to_owned() + base_var_name;
    let args_rep = args
        .iter()
        .filter(|fct_arg| !matches!(fct_arg.get_string_rep_arg_val(), None))
        .map(|fct_arg| fct_arg.get_string_rep_arg_val().as_ref().unwrap().clone())
        .collect::<Vec<String>>()
        .join(", ");
    [
        "try { ",
        &("\tlet ".to_owned()
            + &ret_val_basename
            + " = "
            + base_var_name
            + "."
            + fct_name
            + "("
            + &args_rep
            + ");"),
        &("\tconsole.log({\"".to_owned()
            + &ret_val_basename
            + "\": typeof "
            + &ret_val_basename
            + " == \"function\"? \"[function]\" : "
            + &ret_val_basename
            + ".toString()});"),
        &("\tconsole.log({\"ret_val_type\": typeof ".to_owned() + &ret_val_basename + "});"),
        // rejected promise
        &("\tPromise.resolve(".to_owned()
            + &ret_val_basename
            + ").catch(e => { console.log({\"error\": true}); });"),
        "} catch(e) {",
        "\tconsole.log({\"error\": true});",
        "}",
        "console.log({\"done\": true});",
    ]
    .join("\n")
}
