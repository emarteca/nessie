use crate::module_reps::*; // all the representation structs
use rand::prelude::*;
use std::convert::TryFrom;
use std::path::PathBuf;

pub const DISCOVERY_PHASE_TESTING_BUDGET: i32 = 2;
pub const ALLOW_MULTIPLE_CALLBACK_ARGS: bool = false;
pub const ALLOW_ANY_TYPE_ARGS: bool = false;

/// metadata for the setup required before tests are generated
pub mod SETUP {
    pub const TOY_FS_DIRS: [&str; 2] = ["a/b/test/directory", "a/b/test/dir"];
    pub const TOY_FS_FILES: [&str; 2] = ["a/b/test/directory/file.json", "a/b/file"];
}

pub fn gen_new_sig_with_cb(
    num_args: Option<usize>,
    sigs: &Vec<FunctionSignature>, // TODO don't pick a sig we already picked
    cb_position: i32,
    testgen_db: &mut TestGenDB,
) -> FunctionSignature {
    let num_args = num_args.unwrap_or(5);
    let mut args: Vec<FunctionArgument> = Vec::with_capacity(num_args);

    for arg_index in 0..num_args {
        args.push(if i32::try_from(arg_index) == Ok(cb_position) {
            FunctionArgument::new(ArgType::CallbackType, true, None)
        } else {
            FunctionArgument::new(
                testgen_db
                    .choose_random_arg_type(ALLOW_MULTIPLE_CALLBACK_ARGS, ALLOW_ANY_TYPE_ARGS),
                false,
                None,
            )
        });
    }

    FunctionSignature::new(
        num_args, false, // is async
        args,  // arguments
    )
}

pub struct TestGenDB {
    fs_strings: Vec<PathBuf>,
    rng: rand::prelude::ThreadRng,
}

impl TestGenDB {
    pub fn new() -> Self {
        let rng = thread_rng();
        Self {
            fs_strings: Vec::new(),
            rng,
        }
    }

    pub fn set_fs_strings(&mut self, new_fs_paths: Vec<PathBuf>) {
        self.fs_strings = new_fs_paths;
    }

    /// choose random type for argument
    /// can't have allow_any without allow_cbs
    pub fn choose_random_arg_type(&mut self, allow_cbs: bool, allow_any: bool) -> ArgType {
        // enum ArgType {
        //     NumberType,
        //     StringType,
        //     ArrayType,
        //     ObjectType,
        //     CallbackType,
        //     AnyType,
        // }
        let num_arg_types = 4;
        let max_arg_type_count = num_arg_types
            + if allow_cbs {
                if allow_any {
                    2
                } else {
                    1
                }
            } else {
                0
            };
        match self.rng.gen_range(0..max_arg_type_count) {
            0 => ArgType::NumberType,
            1 => ArgType::StringType,
            2 => ArgType::ArrayType,
            3 => ArgType::ObjectType,
            4 => ArgType::CallbackType,
            _ => ArgType::AnyType,
        }
    }

    pub fn gen_random_value_of_type(&mut self, arg_type: ArgType) -> String {
        match arg_type {
            ArgType::NumberType => 1.to_string(),
            _ => "\"a\"".to_string(), // TODO if string, choose something from the self.fs_strings
        }
    }
}
