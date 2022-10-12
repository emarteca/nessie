use crate::module_reps::*; // all the representation structs for the Npm modules
use crate::testgen::*; // tests and related structs
use rand::{distributions::Alphanumeric, prelude::*};
use std::collections::HashMap;
use std::convert::TryFrom;
use std::path::PathBuf;
use strum::IntoEnumIterator;

pub const DISCOVERY_PHASE_TESTING_BUDGET: i32 = 100;
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
    cb_position: Option<i32>,
    testgen_db: &TestGenDB,
) -> FunctionSignature {
    let num_args = num_args.unwrap_or(DEFAULT_MAX_ARG_LENGTH);
    let mut args: Vec<FunctionArgument> = Vec::with_capacity(num_args);

    for arg_index in 0..num_args {
        args.push(
            if cb_position.is_some() && i32::try_from(arg_index) == Ok(cb_position.unwrap()) {
                FunctionArgument::new(ArgType::CallbackType, true, None)
            } else {
                FunctionArgument::new(
                    testgen_db
                        .choose_random_arg_type(ALLOW_MULTIPLE_CALLBACK_ARGS, ALLOW_ANY_TYPE_ARGS),
                    false,
                    None,
                )
            },
        );
    }

    FunctionSignature::new(
        num_args, &args, // arguments
        None,  // no callback result yet since it wasn't run
    )
}

pub struct TestGenDB {
    fs_strings: Vec<PathBuf>,
    possible_ext_points: Vec<(ExtensionType, (Test, Option<ExtensionPointID>))>,
    cur_test_index: usize,
    pub test_dir_path: String,
    pub test_file_prefix: String,
}

// setup, and generate random values of particular types
impl<'cxt> TestGenDB {
    pub fn new(test_dir_path: String, test_file_prefix: String) -> Self {
        Self {
            fs_strings: Vec::new(),
            possible_ext_points: Vec::new(),
            cur_test_index: 0,
            test_dir_path,
            test_file_prefix,
        }
    }

    pub fn get_test_dir_path(&self) -> String {
        self.test_dir_path.clone()
    }

    pub fn get_test_file_prefix(&self) -> String {
        self.test_file_prefix.clone()
    }

    pub fn set_fs_strings(&mut self, new_fs_paths: Vec<PathBuf>) {
        self.fs_strings = new_fs_paths;
    }

    /// choose random type for argument
    /// can't have allow_any without allow_cbs
    pub fn choose_random_arg_type(&self, allow_cbs: bool, allow_any: bool) -> ArgType {
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
        match thread_rng().gen_range(0..max_arg_type_count) {
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
    pub fn gen_random_value_of_type(&self, arg_type: ArgType) -> String {
        let arg_type = match arg_type {
            ArgType::AnyType => self.choose_random_arg_type(true, false),
            _ => arg_type,
        };
        // TODO!!!! DEBUGGING
        let arg_type = ArgType::CallbackType;
        match arg_type {
            ArgType::NumberType => self.gen_random_number(),
            ArgType::StringType => self.gen_random_string(true),
            ArgType::ArrayType => {
                // to keep things simple, we'll only have arrays of strings and/or numbers, like in the original lambdatester
                // https://github.com/sola-da/LambdaTester/blob/master/utilities/randomGenerator.js#L90
                let num_elts = thread_rng().gen_range(0..MAX_GENERATED_ARRAY_LENGTH);
                let mut gen_array: Vec<String> = Vec::with_capacity(num_elts);
                let array_type = thread_rng().gen_range(0..3);
                for _ in 0..num_elts {
                    gen_array.push(match (array_type, thread_rng().gen_range(0..=1) < 1) {
                        (0, _) | (2, true) => self.gen_random_number(),
                        _ => self.gen_random_string(true),
                    });
                }
                "[".to_owned() + &gen_array.join(", ") + "]"
            }
            ArgType::ObjectType => {
                let num_elts = thread_rng().gen_range(0..MAX_GENERATED_OBJ_LENGTH);
                let mut gen_obj: Vec<String> = Vec::with_capacity(num_elts);
                for _ in 0..num_elts {
                    gen_obj.push(
                        self.gen_random_string(false)
                            + ": "
                            + &match thread_rng().gen_range(0..=1) < 1 {
                                true => self.gen_random_number(),
                                _ => self.gen_random_string(true),
                            },
                    );
                }
                "{".to_owned() + &gen_obj.join(", ") + "}"
            }
            ArgType::CallbackType => {
                let num_args = thread_rng().gen_range(0..DEFAULT_MAX_ARG_LENGTH);
                let cb_position = if num_args == 0 {
                    None
                } else {
                    Some(i32::try_from(thread_rng().gen_range(0..num_args * 2)).unwrap())
                    // x2 means there's a 50% chance of no callback (position never reached)
                    // NOTE: this is for the signature of the callback being generated -- a 
                    // callback is always returned from this branch of the match
                };
                let sigs = Vec::new();
                let random_sig = gen_new_sig_with_cb(Some(num_args), &sigs, cb_position, self);
                self.gen_random_callback(Some(random_sig))
            }
            _ => self.gen_random_string(true),
        }
    }

    /// generate a random number
    fn gen_random_number(&self) -> String {
        (thread_rng().gen_range(-MAX_GENERATED_NUM..MAX_GENERATED_NUM)).to_string()
    }
    /// generate a random string; since we're working with file systems, these strings should sometimes correspond
    /// to valid paths in the operating system
    fn gen_random_string(&self, include_fs_strings: bool) -> String {
        // if string, choose something from the self.fs_strings half the time
        // TODO actually, if we're including fs strings, always choose an fs string
        let string_choice = 0; // self.thread_rng().gen_range(0..=1);
        match (string_choice, include_fs_strings) {
            (0, true) => {
                // choose string from the list of valid files
                let rand_index = thread_rng().gen_range(0..self.fs_strings.len());
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
    /// the `opt_sig` signature should be generated based on the function pool etc
    /// and these should be fields in the generator
    fn gen_random_callback(&self, opt_sig: Option<FunctionSignature>) -> String {
        if let Some(sig) = opt_sig {
            // FIXME! should have some identifier for the cb it's in
            let print_args = sig
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
                &(0..sig.get_arg_list().len())
                    .map(|i| "cb_arg_".to_owned() + &i.to_string())
                    .collect::<Vec<String>>()
                    .join(", "),
                ") => {",
                &print_args,
                "}",
            ]
            .join("\n")
        } else {
            "(...args) => { console.log(args); }".to_string()
        }
    }

    pub fn gen_random_call(&self, mod_rep: &NpmModule) -> FunctionCall {
        let rand_fct_index = mod_rep.get_fns().keys().choose(&mut thread_rng()).unwrap();
        let fct_to_call = &mod_rep.get_fns()[rand_fct_index];
        // TODO! use the fct_to_call.get_sigs() to make a good signature
        let fct_name = fct_to_call.get_name();
        let num_args = if let Some(api_args) = fct_to_call.get_num_api_args() {
            api_args
        } else {
            0
        };
        let cb_position = if num_args == 0 {
            None
        } else {
            Some(i32::try_from(thread_rng().gen_range(0..num_args * 2)).unwrap())
            // x2 means there's a 50% chance of no callback (position never reached)
        };
        let random_sig = gen_new_sig_with_cb(
            fct_to_call.get_num_api_args(),
            fct_to_call.get_sigs(),
            cb_position,
            self,
        );
        let mut ret_call = FunctionCall::new(fct_name, random_sig);
        ret_call.init_args_with_random(self);
        ret_call
    }

    pub fn get_test_to_extend(
        &mut self,
        mod_rep: &'cxt NpmModule,
        ext_type: ExtensionType,
    ) -> (Test, Option<ExtensionPointID>) {
        let rel_exts = self
            .possible_ext_points
            .iter()
            .filter(|(et, _)| et == &ext_type)
            .collect::<Vec<&(ExtensionType, (Test, Option<ExtensionPointID>))>>();
        let rand_test = rel_exts.choose(&mut thread_rng());
        // if there's no valid test to extend yet, then we make a new blank one
        if let Some(test_with_id) = rand_test {
            test_with_id.1.clone()
        } else {
            self.cur_test_index = self.cur_test_index + 1;
            (
                Test::new(
                    mod_rep,
                    self.cur_test_index,
                    self.test_dir_path.clone(),
                    self.test_file_prefix.clone(),
                ),
                None,
            )
        }
    }

    pub fn get_blank_test(&mut self, mod_rep: &'cxt NpmModule) -> Test {
        self.cur_test_index = self.cur_test_index + 1;
        Test::new(
            mod_rep,
            self.cur_test_index,
            self.test_dir_path.clone(),
            self.test_file_prefix.clone(),
        )
    }

    pub fn set_cur_test_index(&mut self, cur_test_index: usize) {
        self.cur_test_index = cur_test_index;
    }

    pub fn add_extension_point(
        &mut self,
        ext_type: ExtensionType,
        test_id: (Test, Option<ExtensionPointID>),
    ) {
        self.possible_ext_points.push((ext_type, test_id));
    }

    pub fn add_extension_points_for_test(
        &mut self,
        test: &Test,
        ext_point_results: &HashMap<ExtensionPointID, FunctionCallResult>,
    ) {
        // a test is only extensible if there are no execution errors
        if ext_point_results
            .values()
            .any(|&res| res == FunctionCallResult::ExecutionError)
        {
            return;
        }
        for (ext_id, res) in ext_point_results.iter() {
            for ext_type in ExtensionType::iter() {
                if res.can_be_extended(ext_type) {
                    self.add_extension_point(ext_type, (test.clone(), Some(*ext_id)));
                }
            }
        }
    }
}
