use crate::consts::*;
use crate::errors::*;
use crate::functions::*;
use crate::mined_seed_reps;
use crate::mined_seed_reps::MinedNestingPairJSON;
use crate::module_reps::*; // all the representation structs for the Npm modules
use crate::tests::*; // tests and related structs
use rand::{
    distributions::{Alphanumeric, WeightedIndex},
    prelude::*,
};
use std::collections::HashMap;
use std::convert::TryFrom;
use std::path::PathBuf;
use strum::IntoEnumIterator;

pub fn gen_new_sig_with_cb(
    num_args: Option<usize>,
    sigs: &Vec<FunctionSignature>, // TODO don't pick a sig we already picked
    cb_position: Option<i32>,
    testgen_db: &TestGenDB,
) -> FunctionSignature {
    // look at the list of signatures CHOOSE_NEW_SIG_PCT of the time (if the list is non-empty)
    if sigs.len() > 0 && (thread_rng().gen_range(0..=100) as f64) / 100. > CHOOSE_NEW_SIG_PCT {
        sigs.choose(&mut thread_rng()).unwrap().clone()
    } else {
        let num_args = num_args.unwrap_or(thread_rng().gen_range(0..=DEFAULT_MAX_ARG_LENGTH));
        let mut args: Vec<FunctionArgument> = Vec::with_capacity(num_args);

        for arg_index in 0..num_args {
            args.push(
                if cb_position.is_some() && i32::try_from(arg_index) == Ok(cb_position.unwrap()) {
                    FunctionArgument::new(ArgType::CallbackType, None)
                } else {
                    FunctionArgument::new(
                        testgen_db.choose_random_arg_type(
                            ALLOW_MULTIPLE_CALLBACK_ARGS,
                            ALLOW_ANY_TYPE_ARGS,
                        ),
                        None,
                    )
                },
            );
        }

        FunctionSignature::new(
            &args, // arguments
            None,  // no callback result yet since it wasn't run
        )
    }
}

pub type LibMinedData = HashMap<String, Vec<MinedNestingPairJSON>>;

pub struct TestGenDB {
    fs_strings: Vec<PathBuf>,
    possible_ext_points: Vec<(
        ExtensionType,
        (Test, Option<ExtensionPointID>, Option<String>),
    )>,
    cur_test_index: usize,
    // keep track of all the functions tested, per library,
    // so we can bias the generator to choose functions that haven't
    // been tested yet
    libs_fcts_weights: HashMap<String, Vec<(String, f64)>>,
    lib_mined_data: LibMinedData,
    pub test_dir_path: String,
    pub test_file_prefix: String,
}

// setup, and generate random values of particular types
impl<'cxt> TestGenDB {
    pub fn new(
        test_dir_path: String,
        test_file_prefix: String,
        mined_data: Option<Vec<MinedNestingPairJSON>>,
    ) -> Self {
        Self {
            fs_strings: Vec::new(),
            possible_ext_points: Vec::new(),
            cur_test_index: 0,
            libs_fcts_weights: HashMap::new(),
            lib_mined_data: match mined_data {
                Some(lmd) => MinedNestingPairJSON::lib_map_from_list(lmd),
                None => HashMap::new(),
            },
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
                    3
                } else {
                    2
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
            5 => ArgType::LibFunctionType,
            _ => ArgType::AnyType,
        }
    }

    /// generate random value of specified argument type
    pub fn gen_random_value_of_type(
        &self,
        arg_type: ArgType,
        arg_pos: Option<usize>,
        ret_vals_pool: &Vec<ArgVal>,
        cb_arg_vals_pool: &Vec<ArgVal>,
        mod_rep: &NpmModule,
    ) -> ArgVal {
        // gen AnyType? only if ret_vals_pool is non-empty
        let arg_type = match (arg_type, (ret_vals_pool.len() + cb_arg_vals_pool.len()) > 0) {
            (ArgType::AnyType, false) => {
                self.choose_random_arg_type(true, false /* no AnyType */)
            }
            (_, _) => arg_type,
        };
        match arg_type {
            ArgType::NumberType => self.gen_random_number_val(),
            ArgType::StringType => self.gen_random_string_val(true),
            ArgType::ArrayType => {
                // to keep things simple, we'll only have arrays of strings and/or numbers, like in the original lambdatester
                // https://github.com/sola-da/LambdaTester/blob/master/utilities/randomGenerator.js#L90
                let num_elts = thread_rng().gen_range(0..=MAX_GENERATED_ARRAY_LENGTH);
                let mut gen_array: Vec<String> = Vec::with_capacity(num_elts);
                let array_type = thread_rng().gen_range(0..=3);
                for _ in 0..num_elts {
                    gen_array.push(match (array_type, thread_rng().gen_range(0..=1) < 1) {
                        (0, _) | (2, true) => self
                            .gen_random_number_val()
                            .get_string_rep(None, None, false),
                        _ => self
                            .gen_random_string_val(true)
                            .get_string_rep(None, None, false),
                    });
                }
                ArgVal::Array("[".to_owned() + &gen_array.join(", ") + "]")
            }
            ArgType::ObjectType => {
                let num_elts = thread_rng().gen_range(0..=MAX_GENERATED_OBJ_LENGTH);
                let mut gen_obj: Vec<String> = Vec::with_capacity(num_elts);
                for _ in 0..num_elts {
                    gen_obj.push(
                        self.gen_random_string_val(false)
                            .get_string_rep(None, None, false)
                            + ": "
                            + &match thread_rng().gen_range(0..=1) < 1 {
                                true => self
                                    .gen_random_number_val()
                                    .get_string_rep(None, None, false),
                                _ => self
                                    .gen_random_string_val(true)
                                    .get_string_rep(None, None, false),
                            },
                    );
                }
                ArgVal::Object("{".to_owned() + &gen_obj.join(", ") + "}")
            }
            ArgType::CallbackType => {
                let num_args = thread_rng().gen_range(0..=DEFAULT_MAX_ARG_LENGTH);
                let cb_position = if num_args == 0 {
                    None
                } else {
                    Some(i32::try_from(thread_rng().gen_range(0..=(num_args * 2))).unwrap())
                    // x2 means there's a 50% chance of no callback (position never reached)
                    // NOTE: this is for the signature of the callback being generated -- a
                    // callback is always returned from this branch of the match
                };
                let sigs = Vec::new();
                let random_sig = gen_new_sig_with_cb(Some(num_args), &sigs, cb_position, self);
                self.gen_random_callback(Some(random_sig), arg_pos)
            }
            ArgType::LibFunctionType => {
                let lib_name = mod_rep.get_mod_js_var_name();
                ArgVal::LibFunction(
                    lib_name + "." + mod_rep.get_fns().keys().choose(&mut thread_rng()).unwrap(),
                )
            }
            ArgType::AnyType => {
                let mut rand_index =
                    thread_rng().gen_range(0..(ret_vals_pool.len() + cb_arg_vals_pool.len()));
                if rand_index < ret_vals_pool.len() {
                    ret_vals_pool
                } else {
                    rand_index = rand_index - ret_vals_pool.len();
                    cb_arg_vals_pool
                }
                .get(rand_index)
                .unwrap()
                .clone()
            }
        }
    }

    /// generate a random number
    fn gen_random_number_val(&self) -> ArgVal {
        ArgVal::Number((thread_rng().gen_range(-MAX_GENERATED_NUM..=MAX_GENERATED_NUM)).to_string())
    }
    /// generate a random string; since we're working with file systems, these strings should sometimes correspond
    /// to valid paths in the operating system
    fn gen_random_string_val(&self, include_fs_strings: bool) -> ArgVal {
        // if string, choose something from the self.fs_strings half the time
        // TODO actually, if we're including fs strings, always choose an fs string
        let string_choice = 0; // self.thread_rng().gen_range(0..=1);
        ArgVal::String(match (string_choice, include_fs_strings) {
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
                        .take(thread_rng().gen_range(1..=RANDOM_STRING_LENGTH))
                        .map(char::from)
                        .collect::<String>()
                    + "\""
            }
        })
    }
    /// generate a random callback
    /// the `opt_sig` signature should be generated based on the function pool etc
    /// and these should be fields in the generator
    fn gen_random_callback(
        &self,
        opt_sig: Option<FunctionSignature>,
        arg_pos: Option<usize>,
    ) -> ArgVal {
        let mut cb = if let Some(sig) = opt_sig {
            Callback::new(sig)
        } else {
            Callback::default()
        };
        cb.set_cb_arg_pos(arg_pos);
        ArgVal::Callback(CallbackVal::RawCallback(cb))
    }

    pub fn gen_random_call(
        &mut self,
        mod_rep: &NpmModule,
        ret_vals_pool: Vec<ArgVal>,
        cb_arg_vals_pool: Vec<ArgVal>,
        ext_facts: (Option<&FunctionCall>, ExtensionType, String),
    ) -> FunctionCall {
        let lib_name = mod_rep.get_mod_js_var_name();

        let (ext_fct, ext_type, ext_uniq_id) = ext_facts;

        // should we try and use mined data?
        if ext_type == ExtensionType::Nested
            && (thread_rng().gen_range(0..=1) as f64) / 100. > USE_MINED_NESTING_EXAMPLE
        {
            let possible_nested_exts = mined_seed_reps::get_rel_mined_data_nested_extensions(
                ext_fct,
                &lib_name,
                &match self.lib_mined_data.get(&lib_name) {
                    Some(lib_list) => lib_list.to_vec(),
                    None => Vec::new(),
                },
            );
            if let Some(nested_ext) = possible_nested_exts.choose(&mut thread_rng()) {
                let ext_fct = ext_fct.unwrap(); // if we can nest, outer fct exists
                let fct_name = nested_ext.fct_name.clone();
                let fct_sig = nested_ext.sig.clone();
                let mut ret_call = FunctionCall::new(
                    fct_name, fct_sig,
                    None, /* position of arg in parent call of cb this is in */
                    None, /* parent call node ID */
                );
                ret_call.init_args_with_random(self, &ret_vals_pool, &cb_arg_vals_pool, mod_rep);
                let mut args = ret_call.sig.get_mut_args();
                // let outer_sig = ext_fct.unwrap().sig;
                // setup the dataflow
                // THIS WILL CHANGE WHEN WE HAVE BETTER MINED DATA
                // right now, the mined data assumes there is only one callback argument to the outer
                // function, and that outer_pos is a valid argument position in this callback
                if ext_fct.sig.get_callback_positions().len() == 1 {
                    let outer_cb_args = ext_fct.sig.get_all_cb_args_vals(&ext_uniq_id);
                    for (outer_pos, inner_pos) in nested_ext.outer_to_inner_dataflow.iter() {
                        if *outer_pos < outer_cb_args.len() {
                            args[*inner_pos] = FunctionArgument::new(
                                ArgType::AnyType,
                                Some(outer_cb_args[*outer_pos].clone()),
                            );
                        }
                    }
                    return ret_call;
                }
            }
        }

        // not using mined data...

        let lib_fcts_weights = self
            .libs_fcts_weights
            .entry(lib_name.clone())
            .or_insert_with(|| {
                mod_rep
                    .get_fns()
                    .keys()
                    .map(|fct_name| (fct_name.clone(), 1.0))
                    .collect()
            });
        let dist =
            WeightedIndex::new(lib_fcts_weights.iter().map(|(fct_name, weight)| weight)).unwrap();
        let rand_fct_index = dist.sample(&mut thread_rng());
        let (fct_name, cur_fct_weight) = &lib_fcts_weights[rand_fct_index].clone();
        let fct_to_call = &mod_rep.get_fns()[fct_name];
        // now update the weight of the function we just picked
        if let Some((fct_name, cur_fct_weight)) = self
            .libs_fcts_weights
            .get_mut(&lib_name)
            .unwrap()
            .get_mut(rand_fct_index)
        {
            *cur_fct_weight = *cur_fct_weight * RECHOOSE_LIB_FCT_WEIGHT_FACTOR;
        }
        let num_args = if let Some(api_args) = fct_to_call.get_num_api_args() {
            api_args
        } else {
            thread_rng().gen_range(0..=DEFAULT_MAX_ARG_LENGTH)
        };
        let cb_position = if num_args == 0 {
            None
        } else {
            Some(i32::try_from(thread_rng().gen_range(0..=(num_args * 2))).unwrap())
            // x2 means there's a 50% chance of no callback (position never reached)
        };
        // choose a random signature -- either new, or an existing one if we know what it is
        let random_sig = gen_new_sig_with_cb(
            fct_to_call.get_num_api_args(),
            fct_to_call.get_sigs(),
            cb_position,
            self,
        );
        let mut ret_call = FunctionCall::new(
            fct_name.clone(),
            random_sig,
            None, /* position of arg in parent call of cb this is in */
            None, /* parent call node ID */
        );
        ret_call.init_args_with_random(self, &ret_vals_pool, &cb_arg_vals_pool, mod_rep);
        ret_call
    }

    pub fn get_test_to_extend(
        &mut self,
        mod_rep: &'cxt NpmModule,
        ext_type: ExtensionType,
    ) -> (Test, Option<ExtensionPointID>, Option<String>) {
        let rel_exts = self
            .possible_ext_points
            .iter()
            .filter(|(et, _)| et == &ext_type)
            .collect::<Vec<&(
                ExtensionType,
                (Test, Option<ExtensionPointID>, Option<String>),
            )>>();
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
        test_id: (Test, Option<ExtensionPointID>, Option<String>),
    ) {
        self.possible_ext_points.push((ext_type, test_id));
    }

    pub fn add_extension_points_for_test(
        &mut self,
        test: &Test,
        ext_point_results: &HashMap<ExtensionPointID, (FunctionCallResult, Option<String>)>,
    ) {
        // a test is only extensible if there are no execution errors
        if ext_point_results
            .values()
            .any(|&(res, _)| res == FunctionCallResult::ExecutionError)
        {
            return;
        }
        for (ext_id, (res, cb_arg_pos)) in ext_point_results.iter() {
            for ext_type in ExtensionType::iter() {
                if res.can_be_extended(ext_type) {
                    self.add_extension_point(
                        ext_type,
                        (test.clone(), Some(*ext_id), cb_arg_pos.clone()),
                    );
                }
            }
        }
    }
}
