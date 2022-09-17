use crate::module_reps::*; // the representation structs, for components
use crate::test_bodies::*;
use crate::decisions::TestGenDB;

use std::process::Command;
use trees::{Tree, Node};
use serde_json::Value;

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
			test_dir_path: self.test_dir_path,
			test_file_prefix: self.test_file_prefix,
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionCall {
	name: String,
	// signature has the arguments
	sig: FunctionSignature,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum FunctionCallNode {
	Call(FunctionCall),
	Root,
}

impl FunctionCall {
	pub fn new(name: String, sig: FunctionSignature) -> Self {
		Self {
			name,
			sig,
		}
	}

	pub fn init_args_with_random(&mut self, testgen_db: &mut TestGenDB) {
		for arg in self.sig.get_mut_args() {
	        let arg_type = arg.get_type();
	        arg.set_string_rep_arg_val(
	            testgen_db.gen_random_value_of_type(arg_type)
	        );
	    }
	}

	pub fn get_code(&self, base_var_name: &str) -> String {
		get_instrumented_function_call(&self.name, base_var_name, &self.sig.get_arg_list())
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Test<'cxt> {
	mod_rep: NpmModule,
	fct_tree: Tree<FunctionCallNode>,
	ext_points: Vec<ExtensionPoint<'cxt>>,
	loc_id: TestLocID,
}

pub type ExtensionPointID = usize;

impl<'cxt> Test<'cxt> {
	pub fn new(mod_rep: NpmModule, cur_test_id: usize, test_dir_path: String, test_file_prefix: String) -> Self {
		let mut fct_tree: Tree<FunctionCallNode> = Tree::new(FunctionCallNode::Root);
		let base_ext_point = ExtensionPoint { node: fct_tree.root(), ext_type: ExtensionType::Sequential};
		Self {
			mod_rep,
			fct_tree,
			ext_points: vec![base_ext_point],
			loc_id: TestLocID {
				cur_test_id,
				test_dir_path,
				test_file_prefix,
			}
		}
	}

	// the testgenDB can deal with random function generation, given a module (which has all the functions)
	// also, the testgenDB will return a test given the extensiontype
	pub fn extend(mod_rep: NpmModule, testgen_db: &mut TestGenDB, ext_type: ExtensionType, new_test_id: usize) -> Result<Self, DFError> {
		let (base_test, ext_id) = testgen_db.get_test_to_extend(&mod_rep, ext_type);
		if base_test.is_empty() && ext_type == ExtensionType::Nested {
			// can't nested extend an empty test
			return Err(DFError::InvalidTestExtensionOption);
		}
		let ext_point_node = base_test.ext_points[ext_id];

		// do the extension
		let ext_call = testgen_db.gen_random_call(mod_rep);
		let ext_node: Tree<FunctionCallNode> = Tree::new(FunctionCallNode::Call(ext_call));
		ext_point_node.node.push_back(ext_node);

		// return the new test
		Ok(Self{
			mod_rep,
			fct_tree: base_test.fct_tree,
			ext_points: Vec::new(), // we don't know what the extension points are yet!
			loc_id: base_test.loc_id.copy_with_new_test_id(new_test_id),
		})
	}

	pub fn is_empty(&self) -> bool {
		// tree is empty if there are no child nodes
		self.fct_tree.degree() == 0
	}

	fn get_code(&self) -> String {
		let setup_code = self.mod_rep.get_js_for_basic_cjs_import();
	    let test_header = get_instrumented_header();
	    let test_footer = get_instrumented_footer();

	    todo!();// also get the string rep for the actual code (tree-walk)

	}

	fn get_file(&self) -> String {
		[self.loc_id.test_dir_path, self.loc_id.test_file_prefix].join("/") + &self.loc_id.cur_test_id.to_string() + ".js"
	}

	fn write_test_to_file(&self) -> Result<String, DFError> {
		let cur_test_file = self.get_file();
		let cur_test = self.get_code();
		if matches!(std::fs::write(&cur_test_file, cur_test), Err(_)) {
                return Err(DFError::WritingTestError);
        }
        Ok(cur_test_file)
	}

	pub fn execute(&mut self, testgen_db: &mut TestGenDB) -> Result<(), DFError>{
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
        let test_results = diagnose_test_correctness(&output_json);
        todo!();
        // if not an error: 
        // add all the nodes as extension points
        // testgen_db.add_extension_point(ext_type: ExtensionType, test_id: (Test, ExtensionPointID))
        // Ok(())
	}
}

// should somehow return a tree of results, that corresponds to the test tree itself
// we can use this to build a list of extension points
// note: we should only extend a test if it has no execution errors
fn diagnose_test_correctness(output_json: &Value) -> Tree<FunctionCallResult> {
	todo!();
}


#[derive(Debug, Eq, PartialEq, Clone)]
pub struct ExtensionPoint<'cxt> {
	node: &'cxt Node<FunctionCallNode>,
	ext_type: ExtensionType,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ExtensionType {
	Sequential,
	Nested
}