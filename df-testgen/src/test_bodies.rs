use crate::module_reps::*; // all the representation structs
use crate::testgen::{Callback, FunctionCall, Test};

impl Callback {
    pub fn get_string_rep(
        &self,
        extra_body_code: Option<String>,
        context_uniq_id: Option<String>,
        print_instrumented: bool,
    ) -> String {
        let print_args = self
            .sig
            .get_arg_list()
            .iter()
            .enumerate()
            .map(|(i, fct_arg)| {
                if print_instrumented {
                    [
                        "\tconsole.log({\"",
                        "in_cb_arg_",
                        &i.to_string(),
                        "_",
                        &match &context_uniq_id {
                            Some(str_id) => str_id.clone(),
                            None => String::new(),
                        },
                        "\": cb_arg_",
                        &i.to_string(),
                        "});",
                    ]
                    .join("")
                } else {
                    String::new()
                }
            })
            .collect::<Vec<String>>()
            .join("\n\t");
        let cb_code = [
            &("(".to_owned()
                + &(0..self.sig.get_arg_list().len())
                    .map(|i| "cb_arg_".to_owned() + &i.to_string())
                    .collect::<Vec<String>>()
                    .join(", ")
                + " ) => {"),
            &print_args,
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

impl Test {
    pub(crate) fn get_code(&self, print_instrumented: bool) -> String {
        let setup_code = self.js_for_basic_cjs_import.clone();
        let (test_header, test_footer) = if print_instrumented {
            (get_instrumented_header(), get_instrumented_footer())
        } else {
            ("", "")
        };

        let base_var_name = self.mod_js_var_name.clone();
        // traverse the tree of function calls and create the test code
        let test_body = self.fct_tree_code(
            base_var_name,
            self.include_basic_callback,
            print_instrumented,
        );

        [test_header, &setup_code, &test_body, test_footer].join("\n")
    }

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
        const ROOT_LEVEL_TABS: usize = 0;
        let mut test_body = self.dfs_print(
            &base_var_name,
            root_node,
            ROOT_LEVEL_TABS,
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
                        ROOT_LEVEL_TABS,
                        include_basic_callback,
                        print_instrumented,
                    );
            }
            next_node = iter.next();
        }
        test_body
    }

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
        let ret_val_basename = "ret_val_".to_owned() + base_var_name;
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
            let mut args_rep = String::new();
            let mut cur_child = cur_root.first_child();
            args.iter()
                .map(|arg| {
                    let extra_body_code =
                        if arg.get_type() == ArgType::CallbackType && cur_child.is_some() {
                            let mut cur_child_node = self.fct_tree.get(cur_child.unwrap()).unwrap();
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
        get_function_call_code(
            &cur_node_call.sig,
            cur_node_call.get_name(),
            args_rep,
            ret_val_basename,
            extra_cb_code,
            base_var_name,
            cur_call_uniq_id,
            indents,
            print_instrumented,
        )
    }
}

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
pub fn get_function_call_code(
    cur_node_call_sig: &FunctionSignature,
    fct_name: &str,
    args_rep: String,
    ret_val_basename: String,
    extra_cb_code: String,
    base_var_name: &str,
    cur_call_uniq_id: String,
    indents: String,
    print_instrumented: bool,
) -> String {
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
        } else {
            String::new()
        }
    };
    let fct_code = [
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
        &(if print_instrumented {
            "\tconsole.log({\"".to_owned()
                + &ret_val_basename
                + "_"
                + &cur_call_uniq_id
                + "\": typeof "
                + &ret_val_basename
                + " == \"function\"? \"[function]\" : "
                + &ret_val_basename
                + ".toString()});"
        } else {
            String::new()
        }),
        &(if print_instrumented {
            ("\tconsole.log({\"ret_val_type_".to_owned()
                + &cur_call_uniq_id
                + "\": typeof "
                + &ret_val_basename
                + "});")
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
