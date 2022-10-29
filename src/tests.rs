use crate::consts;
use crate::decisions::TestGenDB;
use crate::errors::*;
use crate::functions::*;
use crate::module_reps::*; // the representation structs, for components
use serde::{Deserialize, Serialize};

use indextree::Arena;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::process::Command;
use strum_macros::EnumIter;

// tests and test components

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
pub struct FunctionCall {
    name: String,
    // signature has the arguments
    pub(crate) sig: FunctionSignature,
    // argument position in the parent call, of the callback within which this call is nested
    parent_arg_position_nesting: Option<String>,
    parent_call_id: Option<String>,
}

impl FunctionCall {
    pub fn new(
        name: String,
        sig: FunctionSignature,
        parent_arg_position_nesting: Option<String>,
        parent_call_id: Option<ExtensionPointID>,
    ) -> Self {
        Self {
            name,
            sig,
            parent_arg_position_nesting,
            parent_call_id: match parent_call_id {
                Some(id) => Some(id.to_string()),
                None => None,
            },
        }
    }

    pub fn get_parent_call_id(&self) -> Option<String> {
        self.parent_call_id.clone()
    }

    pub fn set_parent_call_id(&mut self, parent_call_id: Option<ExtensionPointID>) {
        self.parent_call_id = match parent_call_id {
            Some(id) => Some(id.to_string()),
            None => None,
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

    pub fn init_args_with_random(
        &mut self,
        testgen_db: &TestGenDB,
        ret_vals_pool: &Vec<ArgVal>,
        cb_arg_vals_pool: &Vec<ArgVal>,
        mod_rep: &NpmModule,
    ) -> Result<(), TestGenError> {
        for (i, arg) in self.sig.get_mut_args().iter_mut().enumerate() {
            let arg_type = arg.get_type();
            arg.set_arg_val(testgen_db.gen_random_value_of_type(
                arg_type,
                Some(i),
                &ret_vals_pool,
                &cb_arg_vals_pool,
                mod_rep,
            ))?;
        }
        Ok(())
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn set_parent_arg_position_nesting(&mut self, parent_arg_position_nesting: Option<String>) {
        self.parent_arg_position_nesting = parent_arg_position_nesting;
    }

    pub fn get_all_cb_args_vals(&self, context_uniq_id: &String) -> Vec<ArgVal> {
        self.sig.get_all_cb_args_vals(context_uniq_id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Test {
    pub(crate) fct_tree: Arena<FunctionCall>,
    ext_points: Vec<ExtensionPoint>,
    loc_id: TestLocID,
    pub(crate) include_basic_callback: bool,
    pub(crate) js_for_basic_cjs_import: String,
    pub(crate) mod_js_var_name: String,
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
        // choose a random test to extend with this new call
        let (mut base_test, ext_id, cb_arg_pos) = testgen_db.get_test_to_extend(&mod_rep, ext_type);
        if (base_test.is_empty() || ext_id.is_none()) && ext_type == ExtensionType::Nested {
            // can't nested extend an empty test
            if !fresh_test_if_cant_extend {
                return Err(DFError::InvalidTestExtensionOption);
            }
        }

        let (ret_vals_pool, cb_arg_vals_pool): (Vec<ArgVal>, Vec<ArgVal>) = if ext_id.is_some() {
            (
                base_test.get_ret_values_accessible_from_ext_point(ext_id.unwrap()),
                base_test.get_cb_arg_values_accessible_from_ext_point(ext_id.unwrap()),
            )
        } else {
            (Vec::new(), Vec::new())
        };

        let (ext_fct, ext_uniq_id): (Option<&FunctionCall>, String) = if ext_id.is_some() {
            (
                Some(base_test.fct_tree.get(ext_id.unwrap()).unwrap().get()),
                base_test.get_uniq_id_for_call(base_test.fct_tree.get(ext_id.unwrap()).unwrap()),
            )
        } else {
            (None, String::new())
        };

        // select random function to call, and create corresponding node
        let mut ext_call = testgen_db.gen_random_call(
            mod_rep,
            ret_vals_pool,
            cb_arg_vals_pool,
            (ext_fct, ext_type, ext_uniq_id),
        );

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
                    // adding a child
                    new_node
                        .get_mut()
                        .set_parent_arg_position_nesting(cb_arg_pos);
                    new_node.get_mut().set_parent_call_id(Some(ext_id));
                    ext_id.append(ext_node_id, &mut base_test.fct_tree);
                }
                ExtensionType::Sequential => {
                    // adding a sibling
                    if let Some(ext_point_parent) = base_test.fct_tree[ext_id].parent() {
                        ext_point_parent.append(ext_node_id, &mut base_test.fct_tree);
                    }
                    // else it's a root's sibling, so won't have a parent
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

    fn get_file(&self) -> String {
        [
            self.loc_id.test_dir_path.clone(),
            self.loc_id.test_file_prefix.clone(),
        ]
        .join("/")
            + &self.loc_id.cur_test_id.to_string()
            + ".js"
    }

    fn write_test_to_file(&self, print_instrumented: bool) -> Result<String, DFError> {
        let cur_test_file = self.get_file();
        let cur_test = self.get_code(print_instrumented);
        if matches!(std::fs::write(&cur_test_file, cur_test), Err(_)) {
            return Err(DFError::WritingTestError);
        }
        Ok(cur_test_file)
    }

    pub fn execute(
        &mut self,
    ) -> Result<HashMap<ExtensionPointID, (FunctionCallResult, Option<String>)>, DFError> {
        let cur_test_file =
            self.write_test_to_file(true /* needs to be instrumented for tracking */)?;

        let timeout = std::time::Duration::from_secs(consts::TEST_TIMEOUT_SECONDS);
        let mut binding = Command::new("timeout");
        let run_test = binding
            .arg(consts::TEST_TIMEOUT_SECONDS.to_string())
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

    pub fn get_uniq_id_for_call(&self, fc: &indextree::Node<FunctionCall>) -> String {
        self.fct_tree.get_node_id(fc).unwrap().to_string()
            + &match &fc.get().parent_call_id {
                Some(pos) => String::from("_pcid".to_owned() + &pos.to_string()),
                None => String::new(),
            }
            + &match &fc.get().parent_arg_position_nesting {
                Some(pos) => String::from("_pos".to_owned() + &pos.to_string()),
                None => String::new(),
            }
    }

    pub fn get_node_id_for_call_data(&self, fc_d: FunctionCall) -> Option<String> {
        for fc in self.fct_tree.iter() {
            if &fc_d == fc.get() {
                return Some(self.get_uniq_id_for_call(fc));
            }
        }
        None
    }

    pub fn get_ret_values_accessible_from_ext_point(
        &self,
        ext_id: ExtensionPointID,
    ) -> Vec<ArgVal> {
        let ext_node = self.fct_tree.get(ext_id).unwrap();
        let ext_node_uniq_id = self.get_uniq_id_for_call(ext_node);

        let ret_base_var_name = "ret_val_".to_owned() + &self.mod_js_var_name.clone();

        self.fct_tree
            .iter()
            .map(|node| self.get_uniq_id_for_call(node))
            .filter(|uniq_id| {
                uniq_id < &ext_node_uniq_id // earlier than current node
                &&
                !ext_node_uniq_id.starts_with(uniq_id) // exclude nesting parents (since they're not accessible in their nest)
                &&
                uniq_id.matches("_").count() == 0 // only the returns from outermost nested functions are available
            })
            .map(|id| ArgVal::Variable(ret_base_var_name.clone() + "_" + &id))
            .collect::<Vec<ArgVal>>()
    }

    pub fn get_cb_arg_values_accessible_from_ext_point(
        &self,
        ext_id: ExtensionPointID,
    ) -> Vec<ArgVal> {
        let ext_node = self.fct_tree.get(ext_id).unwrap();
        let ext_node_uniq_id = self.get_uniq_id_for_call(ext_node);

        // can only use cb args from the direct nesting parents (i.e., the ancestors)
        ext_id
            .ancestors(&self.fct_tree)
            .map(|node_id| {
                let node = self.fct_tree.get(node_id).unwrap();
                // get all the cb args
                let uniq_id = self.get_uniq_id_for_call(node);
                node.get().get_all_cb_args_vals(&uniq_id)
            })
            .flatten()
            .collect::<Vec<ArgVal>>()
    }
}

// should somehow return a tree of results, that corresponds to the test tree itself
// we can use this to build a list of extension points
// note: we should only extend a test if it has no execution errors
fn diagnose_test_correctness(
    test: &Test,
    output_json: &Value,
) -> HashMap<ExtensionPointID, (FunctionCallResult, Option<String>)> {
    let fct_tree = test.get_fct_tree();
    let mut fct_tree_results: HashMap<ExtensionPointID, (FunctionCallResult, Option<String>)> =
        HashMap::new();
    let output_vec = match output_json {
        Value::Array(vec) => vec,
        _ => {
            for fc in fct_tree.iter() {
                fct_tree_results.insert(
                    fct_tree.get_node_id(fc).unwrap(),
                    (FunctionCallResult::ExecutionError, None),
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
                (FunctionCallResult::ExecutionError, None),
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
            let k = &r["callback_exec_".to_owned() + &fc_id];
            if !k.is_null() {
                (callback_pos, cb_arg_pos) = (Some(i), Some(k.to_string()))
            }
        }

        fct_tree_results.insert(
            fct_tree.get_node_id(fc).unwrap(),
            (
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
                cb_arg_pos,
            ),
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
