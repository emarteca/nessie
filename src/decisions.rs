//! All the functionality for generation of random values.
//! This includes `TestGenDB` -- the representation of the state of the current test generation run
//! as this influences the new function calls and tests generated.

use crate::consts::*;
use crate::errors::*;
use crate::functions::*;
use crate::mined_seed_reps;
use crate::mined_seed_reps::{LibMinedData, MinedNestingPairJSON};
use crate::module_reps::*;
use crate::tests::*;
use rand::{
    distributions::{Alphanumeric, WeightedIndex},
    prelude::*,
};
use std::collections::HashMap;
use std::convert::TryFrom;
use std::path::PathBuf;
use strum::IntoEnumIterator;

/// Generate a new signature with `num_args` arguments.
/// `sigs` is a list of previous signatures, and there's a `CHOOSE_SIG_PCT` chance of
/// returning a signature from this list.
/// There's also an optional `cb_position` specifying a position for a callback argument.
/// `testgen_db` is the state of the current test generation run.
pub fn gen_new_sig_with_cb(
    num_args: Option<usize>,
    weighted_sigs: &HashMap<Vec<ArgType>, f64>,
    cb_position: Option<i32>,
    testgen_db: &TestGenDB,
) -> FunctionSignature {
    // look at the list of signatures CHOOSE_NEW_SIG_PCT of the time (if the list is non-empty)
    if weighted_sigs.len() > 0
        && (thread_rng().gen_range(0..=100) as f64) / 100. > CHOOSE_NEW_SIG_PCT
    {
        let vec_sigs_weights = weighted_sigs.iter().collect::<Vec<(&Vec<ArgType>, &f64)>>();
        let dist = WeightedIndex::new(vec_sigs_weights.iter().map(|(_, weight)| **weight)).unwrap();
        let rand_sig_index = dist.sample(&mut thread_rng());
        let (abstract_sig, _) = &vec_sigs_weights[rand_sig_index].clone();
        FunctionSignature::from(*abstract_sig)
    } else {
        let num_args = num_args.unwrap_or(thread_rng().gen_range(0..=DEFAULT_MAX_ARG_LENGTH));
        let mut args: Vec<FunctionArgument> = Vec::with_capacity(num_args);

        // generate random values for all arguments, unless `cb_position` is a valid
        // position (if so, make this a callback).
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

/// Representation of the state of the test generator: configuration for
/// random value generation, informed by previous tests generated/tried.
pub struct TestGenDB {
    /// List of strings representing (valid) paths in the toy filesystem the tests can interact with.
    fs_strings: Vec<PathBuf>,
    /// Base of the toy directory for file system playground
    toy_dir_base: String,
    /// List of possible extension points and types of extension for previous tests.
    possible_ext_points: Vec<(
        ExtensionType,
        (Test, Option<ExtensionPointID>, Option<String>),
    )>,
    /// Current test index.
    cur_test_index: usize,
    /// Keep track of all the functions tested, per library,
    /// so we can bias the generator to choose functions that haven't
    /// been tested yet.
    libs_fcts_weights: HashMap<
        String,
        Vec<(
            (AccessPathModuleCentred, String),
            f64,
            HashMap<Vec<ArgType>, f64>,
        )>,
    >,
    /// Mined data.
    lib_mined_data: LibMinedData,
    /// Directory the generated tests are written to.
    pub test_dir_path: String,
    /// Prefix for the test files (just the file, not the path).
    pub test_file_prefix: String,
    /// Optional: directory of the source code of the package we're generating tests for.
    pub api_src_dir: Option<String>,
}

impl<'cxt> TestGenDB {
    /// Constructor -- initial state of the generator before making any tests.
    pub fn new(
        test_dir_path: String,
        test_file_prefix: String,
        mined_data: Option<Vec<MinedNestingPairJSON>>,
        api_src_dir: Option<String>,
    ) -> Self {
        Self {
            fs_strings: Vec::new(),
            toy_dir_base: String::from("."),
            possible_ext_points: Vec::new(),
            cur_test_index: 0,
            libs_fcts_weights: HashMap::new(),
            lib_mined_data: match mined_data {
                Some(lmd) => MinedNestingPairJSON::lib_map_from_list(lmd),
                None => HashMap::new(),
            },
            test_dir_path,
            test_file_prefix,
            api_src_dir,
        }
    }

    /// Setter for the list of valid toy filesystem paths.
    pub fn set_fs_strings(&mut self, new_fs_paths: Vec<PathBuf>, toy_dir_base: &String) {
        self.fs_strings = new_fs_paths;
        self.toy_dir_base = toy_dir_base.clone();
    }

    /// Choose random type for argument of type `arg_type`.
    /// Note: can't have `allow_any` without `allow_cbs`.
    pub fn choose_random_arg_type(&self, allow_cbs: bool, allow_any: bool) -> ArgType {
        assert!(!(allow_cbs && !allow_any));
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

    /// Generate random value of specified argument type `arg_type`.
    /// Can specify the position of the argument this will correspond to, with `arg_pos` (optional).
    /// `ret_vals_pool` is a list of all the return values from previous function calls that are
    /// in scope here (i.e., can be used as random values); `cb_arg_vals_pool` is the same for
    /// callback argument values.
    /// `mod_rep` is the representation of the API module that this generated value will be a part
    /// of testing: its functions are valid potential random values.
    pub fn gen_random_value_of_type(
        &self,
        arg_type: ArgType,
        arg_pos: Option<usize>,
        ret_vals_pool: &Vec<ArgValAPTracked>,
        cb_arg_vals_pool: &Vec<ArgVal>,
        mod_rep: &NpmModule,
    ) -> ArgVal {
        // gen AnyType? only if `ret_vals_pool` or `cb_arg_vals_pool` is non-empty
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
                let sigs = HashMap::new();
                let random_sig = gen_new_sig_with_cb(Some(num_args), &sigs, cb_position, self);
                self.gen_random_callback(Some(random_sig), arg_pos)
            }
            ArgType::LibFunctionType => {
                // choose a random function in the API
                let lib_name = mod_rep.get_mod_js_var_name();
                ArgVal::LibFunction(
                    lib_name.clone()
                        + "."
                        + mod_rep
                            .get_fns()
                            .keys()
                            .filter(|(fct_acc_path, _)| {
                                fct_acc_path == &AccessPathModuleCentred::RootPath(lib_name.clone())
                            })
                            .map(|(_, fct_name)| fct_name)
                            .choose(&mut thread_rng())
                            .unwrap(),
                )
            }
            ArgType::AnyType => {
                // choose a random value from the pool of available returns/args
                // `AnyType` is only a valid random type if at least one of these lists is non-empty
                let mut rand_index =
                    thread_rng().gen_range(0..(ret_vals_pool.len() + cb_arg_vals_pool.len()));
                if rand_index < ret_vals_pool.len() {
                    ret_vals_pool
                        .iter()
                        .map(|tracked_val| tracked_val.val.clone())
                        .collect::<Vec<ArgVal>>()
                } else {
                    rand_index = rand_index - ret_vals_pool.len();
                    cb_arg_vals_pool.to_vec()
                }
                .get(rand_index)
                .unwrap()
                .clone()
            }
        }
    }

    /// Generate a random number.
    fn gen_random_number_val(&self) -> ArgVal {
        ArgVal::Number((thread_rng().gen_range(-MAX_GENERATED_NUM..=MAX_GENERATED_NUM)).to_string())
    }
    /// Generate a random string.
    /// Since we're possibly working with file system APIs, these strings can be configured to correspond
    /// to valid paths in the operating system with `include_fs_strings`.
    fn gen_random_string_val(&self, include_fs_strings: bool) -> ArgVal {
        // if string, choose something from the self.fs_strings half the time
        // TODO if we're including fs strings, always choose an fs string
        let string_choice = 0; // self.thread_rng().gen_range(0..=1);
        ArgVal::String(match (string_choice, include_fs_strings) {
            (0, true) => {
                // choose string from the list of valid files
                let rand_index = thread_rng().gen_range(0..self.fs_strings.len());
                "\"".to_owned()
                    // if there's an error in the generation of a file path, just return a random string
                    // ... this can happen when testing filesystem APIs, if a function deletes a file
                    + &match std::fs::canonicalize(self.fs_strings[rand_index].clone().into_os_string()) {
                        Ok(s) => s
                        .into_os_string()
                        .into_string()
                        .unwrap(),
                        Err(_) => self.toy_dir_base.clone() + "/" 
                                + &self.gen_random_string_val(false).get_string_rep(None, None, false).replace("\"", ""),}
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
    /// Generate a random callback.
    /// Use the `opt_sig` signature if it's specified, otherwise the default callback.
    /// `arg_pos` is an option to specify the position that this callback is in an arguments list
    /// e.g. if it's `cb` in `some_fct(x, y, cb)` then `arg_pos` would be 2.
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

    /// Generate a random function call, for module `mod_rep`.
    /// `ret_vals_pool` is the list of function return values in scope to be
    /// used in this call (with acc paths rep); `cb_arg_vals_pool` is the same for callback argument
    /// values.
    /// `ext_facts` is a tuple specifying an optional other function call this generated
    /// function will be extending (i.e., parent), along with the extension type and
    /// unique ID for the parent.
    pub fn gen_random_call(
        &mut self,
        mod_rep: &mut NpmModule,
        ret_vals_pool: Vec<ArgValAPTracked>,
        cb_arg_vals_pool: Vec<ArgVal>,
        ext_facts: (Option<&FunctionCall>, ExtensionType, String),
    ) -> Result<FunctionCall, DFError> {
        let lib_name = mod_rep.get_mod_js_var_name().clone();
        let module_root_path = AccessPathModuleCentred::RootPath(lib_name.clone());

        let (ext_fct, ext_type, ext_uniq_id) = ext_facts;

        // should we try and use mined data?
        // TODO: right now we only have mined data relevant for nested extensions,
        // and for functions with the module import as receivers,
        // but this will change.
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
                let fct_acc_path_rep = AccessPathModuleCentred::FieldAccPath(
                    Box::new(module_root_path.clone()),
                    FieldNameType::StringField(fct_name.clone()),
                );
                let mut ret_call = FunctionCall::new(
                    fct_name,
                    fct_sig,
                    None,                   /* position of arg in parent call of cb this is in */
                    None,                   /* parent call node ID */
                    Some(fct_acc_path_rep), /* access path rep of the call */
                    None, /* receiver of the call -- it's the module import by default */
                );
                ret_call.init_args_with_random(self, &ret_vals_pool, &cb_arg_vals_pool, mod_rep)?;
                let args = ret_call.sig.get_mut_args();
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
                    return Ok(ret_call);
                }
            }
        }

        // not using mined data...
        // choose a random function to generate a call for

        // first, get the acc paths in scope
        // the module import is always in scope
        // other than that, it's the ret_vals
        let valid_receivers: Vec<(&ArgVal, &AccessPathModuleCentred)> = ret_vals_pool
            .iter()
            .filter_map(|ap_tracked| match ap_tracked {
                ArgValAPTracked {
                    val,
                    acc_path: Some(ap),
                } => Some((val, ap)),
                _ => None,
            })
            .collect();

        let mut ap_receivers: HashMap<AccessPathModuleCentred, Vec<ArgVal>> = HashMap::new();
        // build a list of return values for each acc path
        for (val, ap) in valid_receivers.iter() {
            ap_receivers
                .entry((**ap).clone())
                .or_insert_with(|| Vec::new())
                .push((**val).clone());
        }
        // add the root module to the valid receivers
        let root_import_val = mod_rep.get_mod_js_var_name().clone();
        ap_receivers.insert(
            AccessPathModuleCentred::RootPath(lib_name.clone()),
            vec![ArgVal::Variable(root_import_val)],
        );

        let lib_fcts_weights: Vec<(
            (&AccessPathModuleCentred, &String, Vec<ArgVal>),
            f64,
            HashMap<Vec<ArgType>, f64>,
        )> = self
            .libs_fcts_weights
            .entry(lib_name.clone())
            .or_insert_with(|| {
                mod_rep
                    .get_fns()
                    .iter()
                    .map(|((fct_acc_path, fct_name), fct_obj)| {
                        (
                            (fct_acc_path.clone(), fct_name.clone()),
                            1.0,
                            fct_obj
                                .get_sigs()
                                .iter()
                                .map(|sig| (sig.get_abstract_sig(), 1.0))
                                .collect::<HashMap<Vec<ArgType>, f64>>(),
                        )
                    })
                    .collect()
            })
            .iter()
            .map(|((fct_acc_path, fct_name), weight, fct_obj)| {
                // get the list of valid receivers with the acc path
                // add this to the lib_fcts_weights. if it's empty change weight to zero
                // note: the root import is always in ap_receivers
                match ap_receivers.get(fct_acc_path) {
                    Some(rec_list) => (
                        (fct_acc_path, fct_name, rec_list.clone()),
                        *weight,
                        fct_obj.clone(),
                    ),
                    _ => (
                        (fct_acc_path, fct_name, Vec::new()),
                        f64::from(0), /* set weight to zero */
                        fct_obj.clone(),
                    ),
                }
            })
            .collect();

        let dist =
            WeightedIndex::new(lib_fcts_weights.iter().map(|(_, weight, _)| weight)).unwrap();
        let rand_fct_index = dist.sample(&mut thread_rng());
        let ((fct_receiver_acc_path, fct_name, receivers), _, fct_sigs_weights) =
            (&lib_fcts_weights[rand_fct_index]).clone();
        let fct_call_receiver = receivers.choose(&mut rand::thread_rng());
        let fct_name = fct_name.clone();
        let fct_to_call = &mod_rep.get_fns()[&(fct_receiver_acc_path.clone(), fct_name.clone())];
        let fct_acc_path_rep = AccessPathModuleCentred::FieldAccPath(
            Box::new(fct_receiver_acc_path.clone()),
            FieldNameType::StringField(fct_name.clone()),
        );

        let num_args = if let Some(api_args) = fct_to_call.get_num_api_args() {
            api_args
        } else {
            thread_rng().gen_range(0..=DEFAULT_MAX_ARG_LENGTH)
        };
        let cb_position = if num_args == 0 {
            None
        } else {
            Some(i32::try_from(thread_rng().gen_range(0..=(num_args * 2))).unwrap())
            // x2 means there's a 50% chance of no callback (position doesnt correspond to valid arg pos)
        };
        // choose a random signature -- either new, or an existing one (if theres some available)
        let random_sig = gen_new_sig_with_cb(
            fct_to_call.get_num_api_args(),
            &fct_sigs_weights,
            cb_position,
            self,
        );

        // now update the weight of the function we just picked, and its signature
        if let Some((_, cur_fct_weight, cur_fct_sig_weights)) = self
            .libs_fcts_weights
            .get_mut(&lib_name)
            .unwrap()
            .get_mut(rand_fct_index)
        {
            *cur_fct_weight = *cur_fct_weight * RECHOOSE_LIB_FCT_WEIGHT_FACTOR;
            *cur_fct_sig_weights
                .entry(random_sig.get_abstract_sig())
                .or_insert(1.0) *= RECHOOSE_FCT_SIG_WEIGHT_FACTOR;
        }

        let mut ret_call = FunctionCall::new(
            fct_name.clone(),
            random_sig,
            None,                   /* position of arg in parent call of cb this is in */
            None,                   /* parent call node ID */
            Some(fct_acc_path_rep), /* access path rep of the fct being called */
            fct_call_receiver.cloned(),
        );
        // init the call with random values of the types specified in `random_sig`
        ret_call.init_args_with_random(self, &ret_vals_pool, &cb_arg_vals_pool, mod_rep)?;
        Ok(ret_call)
    }

    /// Get a test that can be extended with the extension type specified.
    /// If there's no valid test that can be extended, return a new blank one.
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
                    self.api_src_dir.clone(),
                ),
                None,
                None,
            )
        }
    }

    /// Get a blank test for module `mod_rep` (i.e., with no calls).
    pub fn get_blank_test(&mut self, mod_rep: &'cxt NpmModule) -> Test {
        self.cur_test_index = self.cur_test_index + 1;
        Test::new(
            mod_rep,
            self.cur_test_index,
            self.test_dir_path.clone(),
            self.test_file_prefix.clone(),
            self.api_src_dir.clone(),
        )
    }

    /// Set the current test index to `cur_test_index`; future tests will
    /// be generated with this index, which will then be incremented.
    pub fn set_cur_test_index(&mut self, cur_test_index: usize) {
        self.cur_test_index = cur_test_index;
    }

    /// Add an extension point to the list of valid extension points.
    /// Extension points are specified by their type `ext_type` and the
    /// test ID: a tuple of the test, an optional ID for the extension
    /// point this corresponds to, and an option of the position of a
    /// callback argument in this extension point (needed for nested extension).
    fn add_extension_point(
        &mut self,
        ext_type: ExtensionType,
        test_id: (Test, Option<ExtensionPointID>, Option<String>),
    ) {
        self.possible_ext_points.push((ext_type, test_id));
    }

    /// Add all valid extension points for test `test`, given the
    /// results at each of `test`'s extension points in `ext_point_results`.
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
        // for each of the extension points and their results, check if they
        // can be extended with each type of extension.
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
