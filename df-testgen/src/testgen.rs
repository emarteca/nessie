use crate::decisions;
use crate::decisions::TestGenDB;
use crate::module_reps::*; // the representation structs, for components
use crate::test_bodies::*;

use indextree::Arena;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::process::Command;
use strum_macros::EnumIter;

pub fn run_testgen_phase<'cxt>(
    mod_rep: &'cxt mut NpmModule,
    testgen_db: &'cxt mut TestGenDB,
    num_tests: i32,
) -> Result<(), DFError> {
    let mut cur_test_id: usize = 0;
    // if we specify a nested extension but there's no valid test that can be extended
    // in a nested way, don't error, instead just return a fresh test
    const fresh_test_if_cant_extend: bool = true;

    for _ in 0..num_tests {
        // let ext_type: ExtensionType = rand::thread_rng().gen();
        let ext_type = ExtensionType::Nested;

        let (cur_fct_id, mut cur_test) = Test::extend(
            mod_rep,
            testgen_db,
            ext_type,
            cur_test_id,
            fresh_test_if_cant_extend,
        )?;

        let test_results = cur_test.execute()?;

        cur_test_id += 1;
        testgen_db.set_cur_test_index(cur_test_id);
        testgen_db.add_extension_points_for_test(&cur_test, &test_results);
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestLocID {
    pub cur_test_id: usize,
    pub test_dir_path: String,
    pub test_file_prefix: String,
}

impl TestLocID {
    pub fn copy_with_new_test_id(&self, new_test_id: usize) -> Self {
        Self {
            cur_test_id: new_test_id,
            test_dir_path: self.test_dir_path.clone(),
            test_file_prefix: self.test_file_prefix.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Callback {
    sig: FunctionSignature,
    // TODO store the functions that are called in the body of the callback
    // the test tree should also have nodes for callbacks (trace through for child calls)
    inner_calls: Vec<FunctionCall>,
    // unique ID, used when printing to determine what the CB is
    cb_id: Option<String>,
    // the argument position that this callback is in
    cb_arg_pos: Option<usize>,
}

impl Callback {
    pub fn new(sig: FunctionSignature) -> Self {
        Self {
            sig,
            inner_calls: Vec::new(),
            cb_id: None,
            cb_arg_pos: None,
        }
    }

    pub fn get_string_rep(&self) -> String {
        // FIXME! should have some identifier for the cb it's in
        let print_args = self
            .sig
            .get_arg_list()
            .iter()
            .enumerate()
            .map(|(i, fct_arg)| {
                [
                    "\tconsole.log({\"",
                    "in_cb_arg_",
                    &i.to_string(),
                    "\": cb_arg_",
                    &i.to_string(),
                    "});",
                ]
                .join("")
            })
            .collect::<Vec<String>>()
            .join("\n");
        [
            "(",
            &(0..self.sig.get_arg_list().len())
                .map(|i| "cb_arg_".to_owned() + &i.to_string())
                .collect::<Vec<String>>()
                .join(", "),
            ") => {",
            &print_args,
            &[
                "console.log({\"callback_exec_",
                &match &self.cb_id {
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
            .join(""),
            "}",
        ]
        .join("\n")
    }

    pub fn set_cb_id(&mut self, cb_id: Option<String>) {
        self.cb_id = cb_id;
    }

    pub fn set_cb_arg_pos(&mut self, cb_arg_pos: Option<usize>) {
        self.cb_arg_pos = cb_arg_pos;
    }
}

impl std::default::Default for Callback {
    fn default() -> Self {
        Self {
            sig: FunctionSignature::default(),
            inner_calls: Vec::new(),
            cb_id: None,
            cb_arg_pos: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionCall {
    name: String,
    // signature has the arguments
    sig: FunctionSignature,
    // argument position in the parent call, of the callback within which this call is nested
    parent_arg_position_nesting: Option<usize>,
}

impl FunctionCall {
    pub fn new(
        name: String,
        sig: FunctionSignature,
        parent_arg_position_nesting: Option<usize>,
    ) -> Self {
        Self {
            name,
            sig,
            parent_arg_position_nesting,
        }
    }

    pub fn update_cb_args_with_id(&mut self, call_id: usize) -> Result<(), TestGenError> {
        for arg in self.sig.get_mut_args() {
            if arg.get_type() == ArgType::CallbackType {
                arg.set_cb_id(Some(call_id.to_string()))?;
            }
        }
        Ok(())
    }

    pub fn init_args_with_random(&mut self, testgen_db: &TestGenDB) -> Result<(), TestGenError> {
        for (i, arg) in self.sig.get_mut_args().iter_mut().enumerate() {
            let arg_type = arg.get_type();
            arg.set_arg_val(testgen_db.gen_random_value_of_type(arg_type, Some(i)))?;
        }
        Ok(())
    }

    pub fn get_code(
        &self,
        base_var_name: &str,
        cur_call_id: usize,
        include_basic_callback: bool,
    ) -> String {
        get_instrumented_function_call(
            &self.name,
            base_var_name,
            &self.sig,
            cur_call_id,
            self.parent_arg_position_nesting,
            include_basic_callback,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Test {
    fct_tree: Arena<FunctionCall>,
    ext_points: Vec<ExtensionPoint>,
    loc_id: TestLocID,
    include_basic_callback: bool,
    js_for_basic_cjs_import: String,
    mod_js_var_name: String,
}

pub type ExtensionPointID = indextree::NodeId;

impl<'cxt> Test {
    pub fn new(
        mod_rep: &'cxt NpmModule,
        cur_test_id: usize,
        test_dir_path: String,
        test_file_prefix: String,
    ) -> Test {
        Self {
            fct_tree: Arena::new(),
            ext_points: Vec::new(),
            loc_id: TestLocID {
                cur_test_id,
                test_dir_path,
                test_file_prefix,
            },
            include_basic_callback: false,
            js_for_basic_cjs_import: mod_rep.get_js_for_basic_cjs_import(),
            mod_js_var_name: mod_rep.get_mod_js_var_name(),
        }
    }

    // the testgenDB can deal with random function generation, given a module (which has all the functions)
    // also, the testgenDB will return a test given the extensiontype
    pub fn extend(
        mod_rep: &'cxt NpmModule,
        testgen_db: &mut TestGenDB,
        ext_type: ExtensionType,
        new_test_id: usize,
        fresh_test_if_cant_extend: bool,
    ) -> Result<(ExtensionPointID, Test), DFError> {
        // select random function to call, and create corresponding node
        let mut ext_call = testgen_db.gen_random_call(mod_rep);

        // choose a random test to extend with this new call
        let (mut base_test, ext_id) = testgen_db.get_test_to_extend(&mod_rep, ext_type);
        if (base_test.is_empty() || ext_id.is_none()) && ext_type == ExtensionType::Nested {
            // can't nested extend an empty test
            if !fresh_test_if_cant_extend {
                return Err(DFError::InvalidTestExtensionOption);
            }
        } else {
            println!("reee");
        }

        let ext_node_id = base_test.fct_tree.new_node(ext_call);
        // update callback args of ext_call to have the ext_call ID
        let mut new_node = base_test.fct_tree.get_mut(ext_node_id).unwrap();
        new_node
            .get_mut()
            .update_cb_args_with_id(ext_node_id.into());

        // do the extension, if it's a non-empty test
        if ext_id.is_some() {
            let ext_id = ext_id.unwrap();
            match ext_type {
                ExtensionType::Nested => {
                    println!("here!!!: {:?}", new_test_id);
                    // adding a child
                    ext_id.append(ext_node_id, &mut base_test.fct_tree);
                }
                ExtensionType::Sequential => {
                    println!("woop!!!: {:?}", new_test_id);
                    // adding a sibling
                    let ext_point_parent = base_test.fct_tree[ext_id].parent().unwrap_or(ext_id);
                    ext_point_parent.append(ext_node_id, &mut base_test.fct_tree);
                }
            }
        }

        // return the new test
        Ok((
            ext_node_id,
            Self {
                fct_tree: base_test.fct_tree,
                ext_points: Vec::new(), // we don't know what the extension points are yet!
                loc_id: base_test.loc_id.copy_with_new_test_id(new_test_id),
                include_basic_callback: false,
                js_for_basic_cjs_import: base_test.js_for_basic_cjs_import,
                mod_js_var_name: base_test.mod_js_var_name,
            },
        ))
    }

    pub fn test_one_call(
        mod_rep: &NpmModule,
        one_call: FunctionCall,
        include_basic_callback: bool,
        cur_test_id: usize,
        test_dir_path: String,
        test_file_prefix: String,
    ) -> (ExtensionPointID, Test) {
        let mut fct_tree = Arena::new();
        let one_call_id = fct_tree.new_node(one_call);
        (
            one_call_id,
            Self {
                fct_tree,
                ext_points: Vec::new(),
                loc_id: TestLocID {
                    cur_test_id,
                    test_dir_path,
                    test_file_prefix,
                },
                include_basic_callback,
                js_for_basic_cjs_import: mod_rep.get_js_for_basic_cjs_import(),
                mod_js_var_name: mod_rep.get_mod_js_var_name(),
            },
        )
    }

    pub fn is_empty(&self) -> bool {
        self.fct_tree.count() == 0
    }

    fn fct_tree_code(&self, base_var_name: String, include_basic_callback: bool) -> String {
        // no function calls, return the empty string
        if self.is_empty() {
            return String::new();
        }
        // get root
        let mut root_node = self.fct_tree.iter().next().unwrap();
        let mut test_body = self.dfs_print(&base_var_name, root_node, 1, include_basic_callback);

        // then get root siblings
        let mut next_node = root_node.next_sibling();
        while next_node.is_some() {
            root_node = self.fct_tree.get(next_node.unwrap()).unwrap();
            test_body =
                test_body + &self.dfs_print(&base_var_name, root_node, 1, include_basic_callback);
            next_node = root_node.next_sibling();
        }
        test_body
    }

    fn dfs_print(
        &self,
        base_var_name: &str,
        cur_root: &indextree::Node<FunctionCall>,
        num_tabs: usize,
        include_basic_callback: bool,
    ) -> String {
        // get code for current node
        let cur_call_id = self.get_uniq_id_for_call(cur_root);
        let mut cur_code =
            cur_root
                .get()
                .get_code(base_var_name, cur_call_id, include_basic_callback);
        // get children
        let mut cur_child = cur_root.first_child();
        while cur_child.is_some() {
            let mut cur_child_node = self.fct_tree.get(cur_child.unwrap()).unwrap();
            cur_code = [
                cur_code,
                "\t".repeat(num_tabs),
                self.dfs_print(
                    base_var_name,
                    cur_child_node,
                    num_tabs + 1,
                    include_basic_callback,
                ),
                "\n".to_string(),
            ]
            .join("");
            cur_child = cur_child_node.next_sibling();
        }
        cur_code
    }

    fn get_code(&self) -> String {
        let setup_code = self.js_for_basic_cjs_import.clone();
        let test_header = get_instrumented_header();
        let test_footer = get_instrumented_footer();

        let base_var_name = self.mod_js_var_name.clone();
        // traverse the tree of function calls and create the test code
        let test_body = self.fct_tree_code(base_var_name, self.include_basic_callback);

        [test_header, &setup_code, &test_body, test_footer].join("\n")
    }

    fn get_file(&self) -> String {
        [
            self.loc_id.test_dir_path.clone(),
            self.loc_id.test_file_prefix.clone(),
        ]
        .join("/")
            + &self.loc_id.cur_test_id.to_string()
            + ".js"
    }

    fn write_test_to_file(&self) -> Result<String, DFError> {
        let cur_test_file = self.get_file();
        let cur_test = self.get_code();
        if matches!(std::fs::write(&cur_test_file, cur_test), Err(_)) {
            return Err(DFError::WritingTestError);
        }
        Ok(cur_test_file)
    }

    pub fn execute(&mut self) -> Result<HashMap<ExtensionPointID, FunctionCallResult>, DFError> {
        let cur_test_file = self.write_test_to_file()?;

        let timeout = std::time::Duration::from_secs(decisions::TEST_TIMEOUT_SECONDS);
        let mut binding = Command::new("timeout");
        let run_test = binding
            .arg(decisions::TEST_TIMEOUT_SECONDS.to_string())
            .arg("node")
            .arg(&cur_test_file);

        let output = match run_test.output() {
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
        // also, need to update all the extension points if their relevant callbacks were executed
        let test_results = diagnose_test_correctness(self, &output_json);
        Ok(test_results)
    }

    pub fn get_fct_tree(&self) -> &Arena<FunctionCall> {
        &self.fct_tree
    }

    pub fn get_uniq_id_for_call(&self, fc: &indextree::Node<FunctionCall>) -> usize {
        self.fct_tree.get_node_id(fc).unwrap().into()
    }

    pub fn get_node_id_for_call_data(&self, fc_d: FunctionCall) -> Option<usize> {
        for fc in self.fct_tree.iter() {
            if &fc_d == fc.get() {
                return Some(self.get_uniq_id_for_call(fc));
            }
        }
        None
    }
}

// should somehow return a tree of results, that corresponds to the test tree itself
// we can use this to build a list of extension points
// note: we should only extend a test if it has no execution errors
fn diagnose_test_correctness(
    test: &Test,
    output_json: &Value,
) -> HashMap<ExtensionPointID, FunctionCallResult> {
    let fct_tree = test.get_fct_tree();
    let mut fct_tree_results: HashMap<ExtensionPointID, FunctionCallResult> = HashMap::new();
    let output_vec = match output_json {
        Value::Array(vec) => vec,
        _ => {
            for fc in fct_tree.iter() {
                fct_tree_results.insert(
                    fct_tree.get_node_id(fc).unwrap(),
                    FunctionCallResult::ExecutionError,
                );
            }
            return fct_tree_results;
        }
    };
    for fc in fct_tree.iter() {
        let fc_id = test.get_uniq_id_for_call(fc).to_string();
        if matches!(
            output_vec
                .iter()
                .position(|r| r == &json!({"error_".to_owned() + &fc_id: true})),
            Some(_)
        ) {
            fct_tree_results.insert(
                fct_tree.get_node_id(fc).unwrap(),
                FunctionCallResult::ExecutionError,
            );
            return fct_tree_results;
        }
        // now look through and see if the callback was executed
        // and if so, whether or not it was executed sequentially
        let done_pos = output_vec
            .iter()
            .position(|r| r == &json!({"done_".to_owned() + &fc_id: true}));
        let (mut callback_pos, mut cb_arg_pos) = (None, None);
        for (i, r) in output_vec.iter().enumerate() {
            if let k = &r["callback_exec_".to_owned() + &fc_id] {
                if !k.is_null() {
                    (callback_pos, cb_arg_pos) = (Some(i), Some(k))
                }
            }
        }

        // TODO add argpos to the results, so we can see what extension point makes sense
        println!("omfgg: {:?}, {:?}", callback_pos, cb_arg_pos);

        fct_tree_results.insert(
            fct_tree.get_node_id(fc).unwrap(),
            match (done_pos, callback_pos) {
                (Some(done_index), Some(callback_index)) => {
                    // if test ends before callback is done executing, it's async
                    if done_index < callback_index {
                        FunctionCallResult::SingleCallback(
                            SingleCallCallbackTestResult::CallbackCalledAsync,
                        )
                    }
                    // else it's sync
                    else {
                        FunctionCallResult::SingleCallback(
                            SingleCallCallbackTestResult::CallbackCalledSync,
                        )
                    }
                }
                (Some(_), None) => FunctionCallResult::SingleCallback(
                    SingleCallCallbackTestResult::NoCallbackCalled,
                ),
                // if "done" never prints, there was an error
                _ => FunctionCallResult::ExecutionError,
            },
        );
    }
    fct_tree_results
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct ExtensionPoint {
    node_id: ExtensionPointID,
    ext_type: ExtensionType,
}

#[derive(Debug, Clone, Eq, PartialEq, Copy, EnumIter, Rand)]
pub enum ExtensionType {
    Sequential,
    Nested,
}
