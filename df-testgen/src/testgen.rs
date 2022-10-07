use crate::decisions::TestGenDB;
use crate::module_reps::*; // the representation structs, for components
use crate::test_bodies::*;

use indextree::Arena;
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

    // let mut fcts = mod_rep.get_fns().clone();
    let mut test_res_pairs: Vec<(Test, HashMap<ExtensionPointID, FunctionCallResult>)> = Vec::new();

    // for (func_name, func_desc) in fcts.iter_mut() {
    //     let mut cur_cb_position = 1;
    for _ in 0..num_tests {
        let ext_type = ExtensionType::Sequential;
        // let args = gen_args_for_fct_with_cb(&func_desc, Some(cur_cb_position - 1), testgen_db);
        // let fct_call = FunctionCall::new(
        //     func_name.clone(),
        //     FunctionSignature::new(args.len(), &args, None),
        // );

        let (cur_fct_id, mut cur_test) = Test::extend(mod_rep, testgen_db, ext_type, cur_test_id)?;

        let test_results = cur_test.execute()?;

        // let fct_result = test_results.get(&cur_fct_id).unwrap();
        // if fct_result != &FunctionCallResult::ExecutionError {
        //     func_desc.add_sig(FunctionSignature::try_from((&args, *fct_result)).unwrap());
        // }

        // if we haven't tested the current position with no callbacks, do that
        // else, move to the next position in the arg list and try with a callback arg
        // if cur_cb_position < 0 && args.len() > 0 {
        //     cur_cb_position =
        //         (((cur_cb_position * (-1)) + 1) % i32::try_from(args.len()).unwrap()) + 1
        // } else {
        //     cur_cb_position *= -1
        // }
        cur_test_id += 1;
        test_res_pairs.push((cur_test, test_results));
    }
    // }
    testgen_db.set_cur_test_index(cur_test_id);
    for (cur_test, test_results) in test_res_pairs.iter() {
        testgen_db.add_extension_points_for_test(cur_test, test_results);
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionCall {
    name: String,
    // signature has the arguments
    sig: FunctionSignature,
}

impl FunctionCall {
    pub fn new(name: String, sig: FunctionSignature) -> Self {
        Self { name, sig }
    }

    pub fn init_args_with_random(&mut self, testgen_db: &TestGenDB) {
        for arg in self.sig.get_mut_args() {
            let arg_type = arg.get_type();
            arg.set_string_rep_arg_val(testgen_db.gen_random_value_of_type(arg_type));
        }
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
            &self.sig.get_arg_list(),
            cur_call_id,
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
    ) -> Result<(ExtensionPointID, Test), DFError> {
        // select random function to call, and create corresponding node
        let ext_call = testgen_db.gen_random_call(mod_rep);

        // choose a random test to extend with this new call
        let (mut base_test, ext_id) = testgen_db.get_test_to_extend(&mod_rep, ext_type);
        if (base_test.is_empty() || ext_id.is_none()) && ext_type == ExtensionType::Nested {
            // can't nested extend an empty test
            return Err(DFError::InvalidTestExtensionOption);
        }
        println!("{:?}", base_test);

        let ext_node_id = base_test.fct_tree.new_node(ext_call);

        // do the extension, if it's a non-empty test
        if ext_id.is_some() {
            let ext_id = ext_id.unwrap();
            match ext_type {
                ExtensionType::Nested => {
                    ext_id.append(ext_node_id, &mut base_test.fct_tree);
                }
                ExtensionType::Sequential => {
                    // FIXME ellen! make sure this doesn't break if the parent is tree root
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
        while root_node.next_sibling().is_some() {
            root_node = self
                .fct_tree
                .get(root_node.next_sibling().unwrap())
                .unwrap();
            test_body =
                test_body + &self.dfs_print(&base_var_name, root_node, 1, include_basic_callback);
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
        let cur_child = cur_root.first_child();
        if cur_child.is_some() {
            let cur_child_node = self.fct_tree.get(cur_child.unwrap()).unwrap();
            // then get child's siblings (iterating through the children)
            while cur_child_node.next_sibling().is_some() {
                let cur_child_node = self
                    .fct_tree
                    .get(cur_child_node.next_sibling().unwrap())
                    .unwrap();
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
            }
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

    pub fn execute(
        &mut self, /*, testgen_db: &mut TestGenDB*/
    ) -> Result<HashMap<ExtensionPointID, FunctionCallResult>, DFError> {
        let cur_test_file = self.write_test_to_file()?;

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
        // also, need to update all the extension points if their relevant callbacks were executed
        let test_results = diagnose_test_correctness(self, &output_json);
        // todo!();
        // if not an error:
        // add all the nodes as extension points
        // testgen_db.add_extension_point(ext_type: ExtensionType, test_id: (Test, ExtensionPointID))
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
    // TODO! get this to work for multiple calls, to actually return an extension set
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
        let callback_pos = output_vec
            .iter()
            .position(|r| r == &json!({"callback_exec_".to_owned() + &fc_id: true}));

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

#[derive(Debug, Clone, Eq, PartialEq, Copy, EnumIter)]
pub enum ExtensionType {
    Sequential,
    Nested,
}
