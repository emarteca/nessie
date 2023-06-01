//! Representations of the tests and test building components.

use crate::consts;
use crate::decisions::TestGenDB;
use crate::errors::*;
use crate::functions::*;
use crate::module_reps::*;
use crate::TestGenMode;

use indextree::Arena;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::cell::RefCell;
use std::collections::HashMap;
use std::process::Command;
use std::str::FromStr;
use strum_macros::EnumIter;

/// Test identifying information: ID and file path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestLocID {
    /// Test ID.
    pub cur_test_id: usize,
    /// Directory for the test file.
    pub test_dir_path: String,
    /// Prefix for the test file (only the file, not the full path).
    pub test_file_prefix: String,
}

impl TestLocID {
    /// Make a new `TestLocID` copying the location/name, but with
    /// a new ID.
    pub fn copy_with_new_test_id(&self, new_test_id: usize) -> Self {
        Self {
            cur_test_id: new_test_id,
            test_dir_path: self.test_dir_path.clone(),
            test_file_prefix: self.test_file_prefix.clone(),
        }
    }
}

/// Structure of a function call within a test.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionCall {
    /// Name of the function.
    name: String,
    /// Signature of the function; it includes the arguments.
    pub(crate) sig: FunctionSignature,
    /// Argument position in the parent call, of the callback within which this call is nested
    /// (`None` if not a nested call).
    parent_arg_position_nesting: Option<String>,
    /// ID of the parent call (if nested, `None` if not nested).
    parent_call_id: Option<String>,
    /// Access path abstract representation of the function to be called
    acc_path: Option<AccessPathModuleCentred>,
    /// Optional variable representing the receiver (if None, it's the module import)
    pub(crate) receiver: Option<ArgVal>,
}

impl FunctionCall {
    /// Constructor.
    pub fn new(
        name: String,
        sig: FunctionSignature,
        parent_arg_position_nesting: Option<String>,
        parent_call_id: Option<ExtensionPointID>,
        acc_path: Option<AccessPathModuleCentred>,
        receiver: Option<ArgVal>,
    ) -> Self {
        Self {
            name,
            sig,
            parent_arg_position_nesting,
            parent_call_id: parent_call_id.map(|id| id.to_string()),
            acc_path,
            receiver,
        }
    }

    /// Getter for the access path representation of the function being called.
    pub fn get_acc_path(&self) -> &Option<AccessPathModuleCentred> {
        &self.acc_path
    }

    /// Setter for the access path representation of the function being called.
    pub fn set_acc_path(&mut self, acc_path: Option<AccessPathModuleCentred>) {
        self.acc_path = acc_path
    }

    /// Getter for the ID of the nesting parent call.
    pub fn get_parent_call_id(&self) -> Option<String> {
        self.parent_call_id.clone()
    }

    /// Setter for the ID of the nesting parent call.
    pub fn set_parent_call_id(&mut self, parent_call_id: Option<ExtensionPointID>) {
        self.parent_call_id = parent_call_id.map(|id| id.to_string())
    }

    /// Getter for the name of the function.
    pub fn get_name(&self) -> &str {
        &self.name
    }

    /// Setter for the argument position of the callback in the nesting parent call.
    pub fn set_parent_arg_position_nesting(&mut self, parent_arg_position_nesting: Option<String>) {
        self.parent_arg_position_nesting = parent_arg_position_nesting;
    }

    /// Get the list of parameter names for all the callback arguments to this function.
    pub fn get_all_cb_args_vals(&self, context_uniq_id: &String) -> Vec<ArgVal> {
        self.sig.get_all_cb_args_vals(context_uniq_id)
    }

    /// Update the callback arguments of this function to have the specified ID.
    pub fn update_cb_args_with_id(&mut self, call_id: usize) -> Result<(), TestGenError> {
        for arg in self.sig.get_mut_args() {
            if arg.get_type() == ArgType::CallbackType {
                arg.set_cb_id(Some(call_id.to_string()))?;
            }
        }
        Ok(())
    }

    /// Initialize all the arguments in the function signature with random
    /// values, corresponding to their type.
    pub fn init_args_with_random(
        &mut self,
        testgen_db: &TestGenDB,
        ret_vals_pool: &Vec<ArgValAPTracked>,
        cb_arg_vals_pool: &Vec<ArgVal>,
        mod_rep: &NpmModule,
        test_gen_mode: &TestGenMode,
        reset_existing_arg_vals: bool,
    ) -> Result<(), TestGenError> {
        for (i, arg) in self.sig.get_mut_args().iter_mut().enumerate() {
            let arg_type = arg.get_type();
            if !arg.get_arg_val().is_some() || reset_existing_arg_vals {
                arg.set_arg_val(testgen_db.gen_random_value_of_type(
                    arg_type,
                    Some(i),
                    ret_vals_pool,
                    cb_arg_vals_pool,
                    mod_rep,
                    test_gen_mode,
                ))?;
            }
        }
        Ok(())
    }
}

/// Representation of a test.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Test {
    /// Tree of function calls in the test; sibling nodes are
    /// sequential calls and child nodes are nested calls.
    pub(crate) fct_tree: Arena<FunctionCall>,
    /// List of extension points in this test.
    ext_points: Vec<ExtensionPoint>,
    /// ID/location information for this test.
    loc_id: TestLocID,
    /// Whether or not to include the default/basic callback.
    pub(crate) include_basic_callback: bool,
    /// Code for importing the module being tested in this test.
    pub(crate) js_for_basic_cjs_import: String,
    /// Variable representing the import of the module (this is
    /// the root for all the generated library function calls).
    pub(crate) mod_js_var_name: String,
    /// Number of tabs (i.e., indentation level) of the base
    /// level of the test.
    /// Only relevant for the code generation.
    /// Example of why this might not be 0: if the generated test is
    /// to be part of a `mocha` test suite then the body of the test is inside
    /// of a function, so the `root_level_tabs` is 1.
    pub(crate) root_level_tabs: RefCell<usize>,
}

/// ID type for nodes in the test function tree.
pub type ExtensionPointID = indextree::NodeId;

impl<'cxt> Test {
    /// Constructor.
    pub fn new(
        mod_rep: &'cxt NpmModule,
        cur_test_id: usize,
        test_dir_path: String,
        test_file_prefix: String,
        api_src_dir: Option<String>,
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
            js_for_basic_cjs_import: mod_rep.get_js_for_basic_cjs_import(api_src_dir),
            mod_js_var_name: mod_rep.get_mod_js_var_name(),
            root_level_tabs: RefCell::new(0),
        }
    }

    /// Get the test id.
    pub fn get_id(&self) -> usize {
        self.loc_id.cur_test_id
    }

    /// Get the function call corresponding the extension point ID (if one exists, `None` otherwise)
    pub fn get_fct_call_from_id(&self, ext_id: &ExtensionPointID) -> Option<&FunctionCall> {
        match self.fct_tree.get(*ext_id) {
            Some(node) => Some(node.get()),
            None => None,
        }
    }

    /// Create a new test, for a function in the module `mod_rep`, given the test
    /// generation database `testgen_db`, of extension type `ext_type`, with `new_test_id` ID.
    /// The function to be tested in this extension is randomly generated, using `testgen_db`.
    /// The last parameter, `fresh_test_if_cant_extend` specifies the behaviour if there is no
    /// valid test to be extended with the specifications provided: if this parameter is true,
    /// then if there is no valid test to be extended a fresh (one call) test is generated.
    /// Otherwise, a lack of viable extension options would result in an error.
    pub fn extend(
        mod_rep: &'cxt mut NpmModule,
        testgen_db: &mut TestGenDB,
        ext_type: ExtensionType,
        new_test_id: usize,
        fresh_test_if_cant_extend: bool,
        test_gen_mode: &TestGenMode,
    ) -> Result<(ExtensionPointID, Test), DFError> {
        // choose a random test to extend with this new call
        let (mut base_test, ext_id, cb_arg_pos) = testgen_db.get_test_to_extend(mod_rep, ext_type);
        if (base_test.is_empty() || ext_id.is_none()) && ext_type == ExtensionType::Nested {
            // can't nested extend an empty test
            if !fresh_test_if_cant_extend {
                return Err(DFError::InvalidTestExtensionOption);
            }
        }

        // get the return values and callback argument values accessible at the
        // extension point we're extending from (these are part of the valid inputs pool
        // for the function we're about to test)
        let (ret_vals_pool, cb_arg_vals_pool): (Vec<ArgValAPTracked>, Vec<ArgVal>) =
            if let Some(ext_id) = ext_id {
                (
                    base_test.get_ret_values_accessible_from_ext_point(ext_id),
                    base_test.get_cb_arg_values_accessible_from_ext_point(ext_id),
                )
            } else {
                (Vec::new(), Vec::new())
            };

        // info on the function we're extending from
        let (ext_fct, ext_uniq_id): (Option<&FunctionCall>, String) = if let Some(ext_id) = ext_id {
            (
                Some(base_test.fct_tree.get(ext_id).unwrap().get()),
                base_test.get_uniq_id_for_call(base_test.fct_tree.get(ext_id).unwrap()),
            )
        } else {
            (None, String::new())
        };

        // select random function to call, and create corresponding node
        let ext_call = testgen_db.gen_random_call(
            mod_rep,
            ret_vals_pool,
            cb_arg_vals_pool,
            (ext_fct, ext_type, ext_uniq_id),
            test_gen_mode,
        )?;

        let ext_node_id = base_test.fct_tree.new_node(ext_call);
        // update callback args of ext_call to have the ext_call ID
        let new_node = base_test.fct_tree.get_mut(ext_node_id).unwrap();
        new_node
            .get_mut()
            .update_cb_args_with_id(ext_node_id.into())?;

        // do the extension, if it's a non-empty test
        if let Some(ext_id) = ext_id {
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

        let base_test_root_tabs = *base_test.root_level_tabs.borrow();

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
                root_level_tabs: RefCell::new(base_test_root_tabs),
            },
        ))
    }

    /// Generate a test for the one call `one_call` specified.
    pub fn test_one_call(
        mod_rep: &NpmModule,
        one_call: FunctionCall,
        include_basic_callback: bool,
        cur_test_id: usize,
        test_dir_path: String,
        test_file_prefix: String,
        api_src_dir: Option<String>,
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
                js_for_basic_cjs_import: mod_rep.get_js_for_basic_cjs_import(api_src_dir),
                mod_js_var_name: mod_rep.get_mod_js_var_name(),
                root_level_tabs: RefCell::new(0),
            },
        )
    }

    /// Is the test tree empty?
    pub fn is_empty(&self) -> bool {
        self.fct_tree.count() == 0
    }

    /// Getter for the name of the file this test should be printed to;
    /// this is the full path to the file.
    fn get_file(&self) -> String {
        [
            self.loc_id.test_dir_path.clone(),
            self.loc_id.test_file_prefix.clone(),
        ]
        .join("/")
            + &self.loc_id.cur_test_id.to_string()
            + ".js"
    }

    /// Generate the code for this test and write it to the specified file.
    /// Options for instrumenting the test and for printing it as part of a `mocha`
    /// test suite.
    pub fn write_test_to_file(
        &self,
        print_instrumented: bool,
        print_as_test_fct: bool,
    ) -> Result<String, DFError> {
        let cur_test_file = self.get_file();
        let cur_test = self.get_code(print_instrumented, print_as_test_fct);
        if matches!(std::fs::write(&cur_test_file, cur_test), Err(_)) {
            return Err(DFError::WritingTestError(self.get_file().to_string()));
        }
        Ok(cur_test_file)
    }

    pub fn delete_file(&mut self) -> Result<(), DFError> {
        let cur_test_file = self.get_file();
        if matches!(std::fs::remove_file(&cur_test_file), Err(_)) {
            return Err(DFError::DeletingTestError(self.get_file().to_string()));
        }
        Ok(())
    }

    /// Execute the test and return the results for all the extension points.
    /// Test execution includes writing the test out to a file, and dispatching a
    /// call to `nodejs` to run the test.
    /// In addition to the results per extension suite, we also get lists of function properties
    /// for return values with non-primitive types; these can then be added to the list of
    /// functions available for test generation.
    pub fn execute(&mut self) -> Result<TestDiagnostics, DFError> {
        let cur_test_file = self.write_test_to_file(
            true,  /* needs to be instrumented for tracking */
            false, /* running these directly */
        )?;

        let mut binding = Command::new("timeout"); // timeout if the test doesn't terminate within time bound
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
        // and, get the list of new functions available on return values with `ObjectType` type
        let test_results = diagnose_test_correctness(self, &output_json);
        Ok(test_results)
    }

    /// Getter for the function tree.
    pub fn get_fct_tree(&self) -> &Arena<FunctionCall> {
        &self.fct_tree
    }

    /// Get the unique ID for a function call node in the test tree.
    pub fn get_uniq_id_for_call(&self, fc: &indextree::Node<FunctionCall>) -> String {
        self.fct_tree.get_node_id(fc).unwrap().to_string()
            + &match &fc.get().parent_call_id {
                Some(pos) => "_pcid".to_owned() + &pos.to_string(),
                None => String::new(),
            }
            + &match &fc.get().parent_arg_position_nesting {
                Some(pos) => "_pos".to_owned() + &pos.to_string(),
                None => String::new(),
            }
    }

    /// Get the unique ID for the node in the test tree that corresponds to the
    /// function call specified.
    pub fn get_node_id_for_call_data(&self, fc_d: FunctionCall) -> Option<String> {
        for fc in self.fct_tree.iter() {
            if &fc_d == fc.get() {
                return Some(self.get_uniq_id_for_call(fc));
            }
        }
        None
    }

    /// Get the (top-level) library function return values that are accessible at
    /// the extension point specified, along with their access path representations
    /// (wrapped in the `ArgValAPTracked` struct).
    pub fn get_ret_values_accessible_from_ext_point(
        &self,
        ext_id: ExtensionPointID,
    ) -> Vec<ArgValAPTracked> {
        let ext_node = self.fct_tree.get(ext_id).unwrap();
        let ext_node_uniq_id = self.get_uniq_id_for_call(ext_node);

        let ret_base_var_name = "ret_val_".to_owned() + &self.mod_js_var_name.clone();

        self.fct_tree
            .iter()
            .map(|node| (self.get_uniq_id_for_call(node), node.get().get_acc_path()))
            .filter(|(uniq_id, _)| {
                // earlier than current node (alphabetical sort is fine)
                uniq_id < &ext_node_uniq_id
                &&
                // exclude nesting parents (since they're not accessible in their nest)
                !ext_node_uniq_id.starts_with(uniq_id)
                &&
                // only the returns from outermost nested functions are available
                uniq_id.matches('_').count() == 0
            })
            .map(|(id, opt_fct_acc_path_rep)| match opt_fct_acc_path_rep {
                Some(fct_acc_path_rep) => ArgValAPTracked {
                    val: ArgVal::Variable(ret_base_var_name.clone() + "_" + &id),
                    acc_path: Some(AccessPathModuleCentred::ReturnPath(Box::new(
                        fct_acc_path_rep.clone(),
                    ))),
                },
                None => ArgValAPTracked {
                    val: ArgVal::Variable(ret_base_var_name.clone() + "_" + &id),
                    acc_path: None,
                },
            })
            .collect::<Vec<ArgValAPTracked>>()
    }

    /// Get all the callback arguments to (recursive) nesting parents, that are
    /// accessible at the extension point specified.
    pub fn get_cb_arg_values_accessible_from_ext_point(
        &self,
        ext_id: ExtensionPointID,
    ) -> Vec<ArgVal> {
        // can only use cb args from the direct nesting parents (i.e., the ancestors)
        ext_id
            .ancestors(&self.fct_tree)
            .flat_map(|node_id| {
                let node = self.fct_tree.get(node_id).unwrap();
                // get all the cb args
                let uniq_id = self.get_uniq_id_for_call(node);
                node.get().get_all_cb_args_vals(&uniq_id)
            })
            .collect::<Vec<ArgVal>>()
    }
}

pub type TestDiagnostics = (
    HashMap<ExtensionPointID, (FunctionCallResult, Option<String>)>,
    HashMap<AccessPathModuleCentred, Vec<String>>,
);

/// Given the output of running a test, this function parses the output and
/// returns a list of results that corresponds to the test's tree.
/// We can use this to build a list of extension points.
/// Note: we should only extend a test if it has no execution errors; if there
/// are execution errors the test has no valid extension points.
fn diagnose_test_correctness(test: &Test, output_json: &Value) -> TestDiagnostics {
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
            return (fct_tree_results, HashMap::new());
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
            return (fct_tree_results, HashMap::new());
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
    let new_acc_path_fcts = get_function_props_for_acc_paths(output_vec);
    (fct_tree_results, new_acc_path_fcts)
}

/// Get the function properties for a given access path, parsing from the
/// test output (this amounts to looking for an item in the output that is
/// a map item where the key is the access path and the value is the list of
/// properties, and then parsing that).
fn get_function_props_for_acc_paths(
    output_vec: &[Value],
) -> HashMap<AccessPathModuleCentred, Vec<String>> {
    let mut ret_map = HashMap::new();
    // `output_vec` is a list of JSON objects
    for val in output_vec.iter() {
        if let Value::Object(m) = val {
            for (k, val) in m.iter() {
                let acc_path_rep = AccessPathModuleCentred::from_str(k);
                if let Ok(acc_path_rep) = acc_path_rep {
                    if let Value::Array(val_vec) = val {
                        ret_map.insert(
                            acc_path_rep,
                            val_vec
                                .iter()
                                .filter_map(|obj| match obj {
                                    Value::String(s) => Some(s.clone()),
                                    _ => None,
                                })
                                .collect(),
                        );
                    }
                }
            }
        }
    }
    ret_map
}

/// Structure of a test extension point.
#[derive(Debug, Eq, PartialEq, Clone)]
pub struct ExtensionPoint {
    /// ID of the node (function call) at which to extend.
    node_id: ExtensionPointID,
    /// Type of extension that can be made here.
    ext_type: ExtensionType,
}

/// Type of test extension.
#[derive(Debug, Clone, Eq, PartialEq, Copy, EnumIter, Rand)]
pub enum ExtensionType {
    /// Sequential function calls.
    Sequential,
    /// Function calls nested in the callback argument of another function call.
    Nested,
}
