//! Functionality for generating the code for the generated tests.

use crate::functions::*;
use crate::module_reps::{AccessPathModuleCentred, NpmModule};
use crate::tests::{FunctionCall, Test};

/// Code generation for `Callback` objects.
impl Callback {
    /// Get the base of the names for all the parameters of this callback.
    /// These look like `cb_<optional unique ID from surrounding context>_<this callback's position in
    /// the argument list of the function it's being called in>_arg_`.
    /// Each callback parameter starts with this name, followed by the position in the argument list.
    pub(crate) fn get_cb_arg_name_base(&self, context_uniq_id: &Option<String>) -> String {
        "cb_".to_owned()
            + &match context_uniq_id {
                Some(ref id) => id.clone(),
                None => String::new(),
            }
            + &match self.cb_arg_pos {
                Some(id) => "_".to_owned() + &id.to_string(),
                None => String::new(),
            }
            + "_arg_"
    }

    /// Get the string representation of the code of this callback.
    /// Optional parameters for adding `extra_body_code` instrumentation code in the body of the
    /// callback, and `context_uniq_id` to be added as part of the ID of the callback arguments.
    pub fn get_string_rep(
        &self,
        extra_body_code: Option<String>,
        context_uniq_id: Option<String>,
        print_instrumented: bool,
    ) -> String {
        let cb_arg_name_base = self.get_cb_arg_name_base(&context_uniq_id);
        // code to print the values of all the callback arguments;
        // included if we're instrumenting
        let print_args = self
            .sig
            .get_arg_list()
            .iter()
            .enumerate()
            .map(|(i, _)| {
                if print_instrumented {
                    [
                        "\tconsole.log({\"",
                        "in_",
                        &cb_arg_name_base,
                        "_",
                        &i.to_string(),
                        "\": ",
                        &cb_arg_name_base,
                        &i.to_string(),
                        "});",
                    ]
                    .join("")
                } else {
                    String::new()
                }
            })
            .collect::<Vec<String>>()
            .join("\n");
        // body of the callback itself
        let cb_code = [
            // signature
            &("(".to_owned()
                + &(0..self.sig.get_arg_list().len())
                    .map(|i| cb_arg_name_base.clone() + &i.to_string())
                    .collect::<Vec<String>>()
                    .join(", ")
                + " ) => {"),
            // print the argument values
            &print_args,
            // print that the callback is executing
            &if print_instrumented {
                [
                    "\tconsole.log({\"callback_exec_",
                    &match context_uniq_id {
                        Some(str_id) => str_id.clone(),
                        None => String::new(),
                    },
                    "\": ",
                    &match &self.cb_arg_pos {
                        Some(pos_id) => pos_id.to_string(),
                        None => String::new(),
                    },
                    "});",
                ]
                .join("")
            } else {
                String::new()
            },
            // extra code for the callback body (if there's a nested function it
            // is included here)
            &match extra_body_code {
                Some(str) => str.clone(),
                None => String::new(),
            },
            "}",
        ]
        .join("\n");
        cb_code
            .split("\n")
            .filter(|line| line.len() > 0)
            .collect::<Vec<&str>>()
            .join("\n\t")
    }
}

// Code generation for the tests.
impl Test {
    /// Get the code representation for the test;
    /// Options to instrument the test, and to generate the test as a function
    /// that can then be called as part of a `mocha` test suite.
    pub(crate) fn get_code(&self, print_instrumented: bool, print_as_test_fct: bool) -> String {
        let setup_code = self.js_for_basic_cjs_import.clone();
        let (test_header, test_footer) = if print_instrumented {
            (get_instrumented_header(), get_instrumented_footer())
        } else {
            ("", "")
        };

        let (test_fct_header, test_fct_footer) = if print_as_test_fct {
            *self.root_level_tabs.borrow_mut() = 1;
            ("module.exports = function() {", "}")
        } else {
            *self.root_level_tabs.borrow_mut() = 0;
            ("", "")
        };

        let base_var_name = self.mod_js_var_name.clone();
        // traverse the tree of function calls and create the test code
        let test_body = self.fct_tree_code(
            base_var_name,
            self.include_basic_callback,
            print_instrumented,
        );

        [
            test_header,
            &setup_code,
            test_fct_header,
            &test_body,
            test_fct_footer,
            test_footer,
        ]
        .join("\n")
    }

    /// Get the code for the tree of function calls in the test.
    fn fct_tree_code(
        &self,
        base_var_name: String,
        include_basic_callback: bool,
        print_instrumented: bool,
    ) -> String {
        // no function calls, return the empty string
        if self.is_empty() {
            return String::new();
        }
        // get root
        let mut iter = self.fct_tree.iter();
        let mut root_node = iter.next().unwrap();
        let mut test_body = self.dfs_print(
            &base_var_name,
            root_node,
            *self.root_level_tabs.borrow(),
            include_basic_callback,
            print_instrumented,
        );

        // then get root siblings
        let mut next_node = iter.next();
        while next_node.is_some() {
            root_node = next_node.unwrap();
            // if it's a root node sibling
            if root_node.parent().is_none() {
                test_body = test_body
                    + &self.dfs_print(
                        &base_var_name,
                        root_node,
                        *self.root_level_tabs.borrow(),
                        include_basic_callback,
                        print_instrumented,
                    );
            }
            next_node = iter.next();
        }
        test_body
    }

    /// Recursively generate the code for a function call in the test's function tree.
    fn dfs_print(
        &self,
        base_var_name: &str,
        cur_root: &indextree::Node<FunctionCall>,
        num_tabs: usize,
        include_basic_callback: bool,
        print_instrumented: bool,
    ) -> String {
        let cur_call_uniq_id = self.get_uniq_id_for_call(cur_root);
        let cur_call_node_id = self.fct_tree.get_node_id(cur_root).unwrap();

        let cur_node_call = cur_root.get();

        let indents = "\t".repeat(num_tabs);
        let ret_val_basename = "ret_val_".to_owned() + base_var_name + "_" + &cur_call_uniq_id;
        let ret_val_acc_path = match cur_node_call.get_acc_path() {
            Some(fct_acc_path_rep) => Some(AccessPathModuleCentred::ReturnPath(Box::new(
                fct_acc_path_rep.clone(),
            ))),
            None => None,
        };
        let extra_cb_code = if include_basic_callback {
            basic_callback_with_id(cur_call_uniq_id.clone())
        } else {
            String::new()
        };
        let args_rep = if cur_node_call.sig.is_spread_args {
            "...args".to_string()
        } else {
            // iterate over the arguments to the current call, and if they're callbacks
            // print those bodies accordingly -- this is equivalent to
            // iterating through the children
            let args = cur_node_call.sig.get_arg_list();
            let mut cur_child = cur_root.first_child();
            args.iter()
                .map(|arg| {
                    let extra_body_code =
                        if arg.get_type() == ArgType::CallbackType && cur_child.is_some() {
                            let cur_child_node = self.fct_tree.get(cur_child.unwrap()).unwrap();
                            let ret_val = if cur_child_node.get().get_parent_call_id()
                                == Some(cur_call_node_id.to_string())
                            {
                                Some(
                                    [
                                        self.dfs_print(
                                            base_var_name,
                                            cur_child_node,
                                            num_tabs + 1,
                                            include_basic_callback,
                                            print_instrumented,
                                        ),
                                        "\n".to_string(),
                                    ]
                                    .join(""),
                                )
                            } else {
                                None
                            };
                            cur_child = cur_child_node.next_sibling();
                            ret_val
                        } else {
                            None
                        };
                    arg.get_string_rep_arg_val(
                        extra_body_code,
                        Some(cur_call_uniq_id.clone()),
                        print_instrumented,
                    )
                    .as_ref()
                    .unwrap()
                    .clone()
                })
                .collect::<Vec<String>>()
                .join(", ")
        };
        assert!(matches!(
            cur_node_call.receiver,
            None | Some(ArgVal::Variable(_))
        )); // receiver needs to be a variable
        let fct_call_base_var = match &cur_node_call.receiver {
            Some(rec) => rec.get_string_rep(None, None, print_instrumented),
            None => base_var_name.to_string(),
        };
        get_function_call_code(
            &cur_node_call.sig,
            cur_node_call.get_name(),
            args_rep,
            (ret_val_basename, ret_val_acc_path),
            extra_cb_code,
            &fct_call_base_var,
            cur_call_uniq_id,
            indents,
            print_instrumented,
        )
    }
}

/// Code generation for modules.
impl NpmModule {
    /// Return JS code to import this module.
    pub fn get_js_for_basic_cjs_import(&self, api_src_dir: Option<String>) -> String {
        [
            "let ",
            &self.get_mod_js_var_name(),
            " = ",
            &match &self.import_code {
                Some(code) => code.clone(),
                None => [
                    "require(\"",
                    &match api_src_dir {
                        Some(dir) => dir,
                        None => self.lib.clone(),
                    },
                    "\")",
                ]
                .join(""),
            },
            ";",
        ]
        .join("")
    }
}

/// Simplest callback: just print that it has executed.
pub fn basic_callback() -> &'static str {
    r#"let cb = function() { console.log({"callback_exec": true}); }"#
}

/// Small variation on the basic callback that includes a unique ID
/// to print when it is executed.
pub fn basic_callback_with_id(cur_call_uniq_id: String) -> String {
    "let cb = function() { console.log({\"callback_exec_".to_owned()
        + &cur_call_uniq_id
        + "\": true}); }"
}

/// Returns a string of JS code that redefines the `console.log`
/// printing function so that it pushes the argument to `console.log`
/// onto an array.
/// This instrumentation allows us to track what's being printed and
/// in what order.
pub fn get_instrumented_header() -> &'static str {
    r#"
let orig_log = console.log;
let output_log = [];
console.log = function(e) {
	output_log.push(e);
}
function getTypeDiffObjFromPromise(val) {
    if (val.toString() === "[object Promise]") {
        return "DIFFTYPE_Promise";
    }
    return typeof val;
}
"#
}

/// Returns a string of JS code that prints the global array that
/// `console.log` is redefined to add to, to the console on process exit
/// (if there are async functions, this will be after all the async functions
/// have finished executing).
pub fn get_instrumented_footer() -> &'static str {
    r#"
process.on("exit", function f() {
	orig_log(JSON.stringify(output_log));
})"#
}

/// Generate the code for a given function call:
/// -- wrap the given call in a try-catch, to catch runtime errors
/// -- print the return value, and the argument values before the call
/// -- print an error in the catch
/// Remember that "print" (i.e., `console.log`) has been redefined to push to the
/// global array on the `process` object.
pub fn get_function_call_code(
    cur_node_call_sig: &FunctionSignature,
    fct_name: &str,
    args_rep: String,
    (ret_val_basename, ret_val_acc_path): (String, Option<AccessPathModuleCentred>),
    extra_cb_code: String,
    base_var_name: &str,
    cur_call_uniq_id: String,
    indents: String,
    print_instrumented: bool,
) -> String {
    // print the arguments to the specified signature
    let print_args = |title: String| {
        if print_instrumented {
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
                                .get_string_rep_arg_val_short()
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
        } else {
            String::new()
        }
    };
    let fct_code = [
        &("let ".to_owned() + &ret_val_basename + ";"),
        "try { ",
        &extra_cb_code,
        &print_args("before_cb".to_string()),
        &("\t".to_owned()
            + &ret_val_basename
            + " = "
            + base_var_name
            + "."
            + &fct_name
            + "("
            + &args_rep
            + ");"),
        &print_args("after_cb".to_string()),
        // print the list of function properties on the acc path if it's an Object type
        // note: we're deliberately ignoring primitives, can explicitly code those cases
        // in if we want (eg for promise chains), but if you want to test all function props
        // on an acc path regardless of type just remove the if statement
        &(if print_instrumented && ret_val_acc_path.is_some() {
            "\tif (getTypeDiffObjFromPromise(".to_owned()
                + &ret_val_basename
                + ") == \"object\"){"
                + "\n\t\tconsole.log({\""
                + &ret_val_acc_path
                    .as_ref()
                    .unwrap()
                    .to_string()
                    .replace("\"", "\\\"")
                + "\": Object.getOwnPropertyNames("
                + &ret_val_basename
                + ").filter((p) => typeof ret_val_jsonfile_1[p] === \"function\")"
                // NOTE: the next lines get more properties; including `toString` etc. 
                // uncomment if you want the prototype properties too
                // + ".concat(Object.getOwnPropertyNames(Object.getPrototypeOf("
                // + &ret_val_basename
                // + ")))"
                + "});"
                // special case for promises: we only want `then` and `catch`
                + "\n\t} else if (getTypeDiffObjFromPromise("
                + &ret_val_basename
                + ") == \"DIFFTYPE_Promise\"){"
                + "\n\t\tconsole.log({\""
                + &ret_val_acc_path
                    .as_ref()
                    .unwrap()
                    .to_string()
                    .replace("\"", "\\\"")
                + "\": [\"then\", \"catch\"]});"
                + "\n\t}"
        } else {
            String::new()
        }),
        &(if print_instrumented {
            "\tconsole.log({\"".to_owned()
                + &ret_val_basename
                + "\": getTypeDiffObjFromPromise("
                + &ret_val_basename
                + ") == \"function\"? \"[function]\" : "
                + &ret_val_basename
                + ".toString()});"
        } else {
            String::new()
        }),
        &(if print_instrumented {
            "\tconsole.log({\"".to_owned()
                + &ret_val_basename
                + "_type\": getTypeDiffObjFromPromise("
                + &ret_val_basename
                + ")});"
        } else {
            String::new()
        }),
        &(if print_instrumented && ret_val_acc_path.is_some() {
            "\tconsole.log({\"".to_owned()
                + &ret_val_basename
                + "_acc_path\": \""
                + &ret_val_acc_path
                    .as_ref()
                    .unwrap()
                    .to_string()
                    .replace("\"", "\\\"")
                + "\"});"
        } else {
            String::new()
        }),
        // rejected promise
        &("\tPromise.resolve(".to_owned()
            + &ret_val_basename
            + ").catch(e => { console.log({\"error_"
            + &cur_call_uniq_id
            + "\": true}); });"),
        "} catch(e) {",
        &("\tconsole.log({\"error_".to_owned() + &cur_call_uniq_id + "\": true});"),
        "}",
        &(if print_instrumented {
            "console.log({\"done_".to_owned() + &cur_call_uniq_id + "\": true});"
        } else {
            String::new()
        }),
    ]
    .join("\n");

    ("\n".to_owned() + &indents)
        + &fct_code
            .split("\n")
            .into_iter()
            .filter(|line| line.len() > 0)
            .collect::<Vec<&str>>()
            .join(&("\n".to_owned() + &indents))
}

/// Generate the code for the `mocha` test suite driver
/// for `num_tests` number of generated tests.
pub fn get_meta_test_code(num_tests: i32) -> String {
    // async error handler -- this avoids the test suite bailing out early if
    // there is an error in one of the tests
    let mut ret_code = [
        "if (!process.hasUncaughtExceptionCaptureCallback()) process.setUncaughtExceptionCaptureCallback(() => {",
        "\tconsole.log(\"{\\\"async_error_in_test\\\": true}\");",
        "});",          
    ].join("\n");
    for i in 1..=num_tests {
        ret_code.push_str(
            &[
                &("\ndescribe('test".to_owned() + &i.to_string() + "!', function () {"),
                "\tit('', async () => {",
                &("\t\tawait require('./test".to_owned() + &i.to_string() + ".js')();"),
                "\t});\n});",
            ]
            .join("\n"),
        );
    }
    ret_code
}
