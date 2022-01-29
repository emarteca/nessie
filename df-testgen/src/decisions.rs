use crate::module_reps::*; // all the representation structs
use rand::{distributions::Alphanumeric, prelude::*};
use std::convert::TryFrom;
use std::path::PathBuf;

pub const DISCOVERY_PHASE_TESTING_BUDGET: i32 = 2;
pub const ALLOW_MULTIPLE_CALLBACK_ARGS: bool = false;
pub const ALLOW_ANY_TYPE_ARGS: bool = false;

pub const MAX_GENERATED_NUM: f64 = 1000.0;
pub const MAX_GENERATED_ARRAY_LENGTH: usize = 10;
pub const MAX_GENERATED_OBJ_LENGTH: usize = 5;
pub const RANDOM_STRING_LENGTH: usize = 5;
pub const DEFAULT_MAX_ARG_LENGTH: usize = 5;

/// metadata for the setup required before tests are generated
pub mod setup {
    pub const TOY_FS_DIRS: [&str; 2] = ["a/b/test/directory", "a/b/test/dir"];
    pub const TOY_FS_FILES: [&str; 2] = ["a/b/test/directory/file.json", "a/b/file"];
}

pub fn gen_new_sig_with_cb(
    num_args: Option<usize>,
    _sigs: &Vec<FunctionSignature>, // TODO don't pick a sig we already picked
    cb_position: i32,
    testgen_db: &mut TestGenDB,
) -> FunctionSignature {
    let num_args = num_args.unwrap_or(DEFAULT_MAX_ARG_LENGTH);
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
        num_args, args, // arguments
        None, // no callback result yet since it wasn't run
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

    /// generate random value of specified argument type
    /// return a string representation of the JS equivalent
    pub fn gen_random_value_of_type(&mut self, arg_type: ArgType) -> String {
        let arg_type = match arg_type {
            ArgType::AnyType => self.choose_random_arg_type(true, false),
            _ => arg_type,
        };
        match arg_type {
            ArgType::NumberType => self.gen_random_number(),
            ArgType::StringType => self.gen_random_string(true),
            ArgType::ArrayType => {
                // to keep things simple, we'll only have arrays of strings and/or numbers, like in the original lambdatester
                // https://github.com/sola-da/LambdaTester/blob/master/utilities/randomGenerator.js#L90
                let num_elts = self.rng.gen_range(0..MAX_GENERATED_ARRAY_LENGTH);
                let mut gen_array: Vec<String> = Vec::with_capacity(num_elts);
                let array_type = self.rng.gen_range(0..3);
                for _ in 0..num_elts {
                    gen_array.push(match (array_type, self.rng.gen_range(0..=1) < 1) {
                        (0, _) | (2, true) => self.gen_random_number(),
                        _ => self.gen_random_string(true),
                    });
                }
                "[".to_owned() + &gen_array.join(", ") + "]"
            }
            ArgType::ObjectType => {
                let num_elts = self.rng.gen_range(0..MAX_GENERATED_OBJ_LENGTH);
                let mut gen_obj: Vec<String> = Vec::with_capacity(num_elts);
                for _ in 0..num_elts {
                    gen_obj.push(
                        self.gen_random_string(false)
                            + ": "
                            + &match self.rng.gen_range(0..=1) < 1 {
                                true => self.gen_random_number(),
                                _ => self.gen_random_string(true),
                            },
                    );
                }
                "{".to_owned() + &gen_obj.join(", ") + "}"
            }
            ArgType::CallbackType => self.gen_random_callback(),
            _ => self.gen_random_string(true),
        }
    }

    /// generate a random number
    fn gen_random_number(&mut self) -> String {
        (self.rng.gen_range(-MAX_GENERATED_NUM..MAX_GENERATED_NUM)).to_string()
    }
    /// generate a random string; since we're working with file systems, these strings should sometimes correspond
    /// to valid paths in the operating system
    fn gen_random_string(&mut self, include_fs_strings: bool) -> String {
        // if string, choose something from the self.fs_strings half the time
        let string_choice = self.rng.gen_range(0..=1);
        match (string_choice, include_fs_strings) {
            (0, true) => {
                // choose string from the list of valid files
                let rand_index = self.rng.gen_range(0..self.fs_strings.len());
                "\"".to_owned()
                    + &self.fs_strings[rand_index]
                        .clone()
                        .into_os_string()
                        .into_string()
                        .unwrap()
                    + "\""
            }
            _ => {
                // choose a random string
                "\"".to_owned()
                    + &rand::thread_rng()
                        .sample_iter(&Alphanumeric)
                        .take(RANDOM_STRING_LENGTH)
                        .map(char::from)
                        .collect::<String>()
                    + "\""
            }
        }
    }
    /// generate a random callback
    /// TODO right now there's just one option, a function that prints its arguments
    fn gen_random_callback(&mut self) -> String {
        "(...args) => { console.log(args); }".to_string()
    }
}
