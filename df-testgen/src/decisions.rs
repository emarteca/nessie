use crate::module_reps::*; // all the representation structs
use std::path::PathBuf;


pub const DISCOVERY_PHASE_TESTING_BUDGET: i32 = 1;

/// metadata for the setup required before tests are generated
pub mod SETUP {
	pub const TOY_FS_DIRS: [&str; 2] = ["a/b/test/directory", "a/b/test/dir"];
	pub const TOY_FS_FILES: [&str; 2] = ["a/b/test/directory/file.json", "a/b/file"];
}

pub fn gen_new_sig(num_args: Option<i32>, sigs: &Vec<FunctionSignature>) -> FunctionSignature {
	let num_args = num_args.unwrap_or(5);
	FunctionSignature::new(
		num_args, 
		false, // is async 
		Vec::new() // arguments
	)
}

pub struct TestGenDB {
	fs_strings: Vec<PathBuf>,
}

impl TestGenDB {
	pub fn new() -> Self {
		Self {
			fs_strings: Vec::new(),
		}
	}

	pub fn set_fs_strings(&mut self, new_fs_paths: Vec<PathBuf>) {
		self.fs_strings = new_fs_paths;
	}
}
