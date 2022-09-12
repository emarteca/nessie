use crate::module_reps::*; // the representation structs, for components

use std::process::Command;
use trees::{Tree, Node};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestLocID {
	pub cur_test_id: usize,
	pub test_dir_path: String,
	pub test_file_prefix: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Test {
	mod_rep: NpmModule,
	fct_tree: Tree,
	ext_points: Vec<ExtensionPoint>;
	loc_id: TestLocID,
}

pub type ExtensionPointID = usize;

impl Test {
	pub fn new(mod_rep: NpmModule, cur_test_id: usize, test_dir_path: String, test_file_prefix: String) {
		Self {
			mod_rep,
			fct_tree: Tree::new(),
			ext_points: Vec::new(),
			loc_id: TestLocID {
				cur_test_id,
				test_dir_path,
				test_file_prefix,
			}
		}
	}

	// the testgenDB can deal with random function generation, given a module (which has all the functions)
	// also, the testgenDB will return a test given the extensiontype
	pub fn extend(mod_rep: NpmModule, testgen_db: &mut TestGenDB, ext_type: ExtensionType) -> Result<Self, DFError> {
		let (base_test, ext_id) = testgen_db.get_test_to_extend(mod_rep, ext_type);
		let ext_point = if base_test.is_empty() {
			if ext_type == ExtensionType::Nested {
				// can't nested extend an empty test
				return Err(DFError::InvalidTestExtensionOption);
			}
			base_test.fct_tree.root()
		} else {
			base_test.ext_points[ext_id]
		};

		// do the extension

		// return the new test
		Ok(Self{

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
		"/".join([self.loc_id.test_dir_path, self.loc_id.test_file_prefix]) + &cur_test_id.to_string() + ".js"
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
        let test_result = diagnose_single_callback_correctness(&output_json);
        if test_result != SingleCallCallbackTestResult::ExecutionError {
            todo!(); // add the new node as an extension point
            // testgen_db.add_extension_point(ext_type: ExtensionType, test_id: (Test, ExtensionPointID))
        }
        Ok(())
	}

}

#[derive(Debug, Clone)]
pub struct ExtensionPoint {
	node: Node,
	ext_type: ExtensionType,
}

#[derive(Debug, Clone)]
pub enum ExtensionType {
	Sequential,
	Nested
}