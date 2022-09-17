/// the data structures representing a module
use std::collections::HashMap;
use std::convert::TryFrom;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// serializable representation of the module
/// at the api_info stage (i.e., only statically looked at the library)
#[derive(Debug, Serialize, Deserialize)]
struct NpmModuleJSON {
    /// name of the npm module
    lib: String,
    /// map of functions making up the module
    /// indexed by the name of the function
    /// here the functions are the output of the api_info phase
    fns: HashMap<String, ModFctAPIJSON>,
}

/// serializable representation of the function as discovered by the api_info
/// this is just static info from the api
#[derive(Debug, Serialize, Deserialize)]
struct ModFctAPIJSON {
    /// name of the function
    name: String,
    /// number of arguments
    num_args: usize,
    /// indicator of whether or not the API function has a specified
    /// number of args
    used_default_args: Option<bool>,
}

/// Module class:
/// - represents the library
/// - composed of a list of functions
/// - each function is composed of a list of signatures
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct NpmModule {
    /// name of the npm module
    lib: String,
    /// map of functions making up the module
    /// indexed by the name of the function
    fns: HashMap<String, ModuleFunction>,
}

/// pretty printing for the NpmModule (JSON style)
impl std::fmt::Debug for NpmModule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match serde_json::to_string_pretty(&self) {
            Ok(pretty_json) => write!(f, "{}", pretty_json),
            _ => Err(std::fmt::Error),
        }
    }
}

impl NpmModule {
    pub fn get_mut_fns(&mut self) -> &mut HashMap<String, ModuleFunction> {
        &mut self.fns
    }

    pub fn get_fns(&self) -> &HashMap<String, ModuleFunction> {
        &self.fns
    } 

    /// create an NpmModule object from a JSON file resulting from running the api_info
    /// phase: this is just a list of all the functions for a module, without having
    /// run the discovery phase yet (i.e., no arg info yet)
    pub fn from_api_spec(path: PathBuf, _mod_name: String) -> Result<Self, DFError> {
        let file_conts = std::fs::read_to_string(path);
        let file_conts_string = match file_conts {
            Ok(fcs) => fcs,
            _ => return Err(DFError::SpecFileError),
        };

        let mod_json_rep: NpmModuleJSON = match serde_json::from_str(&file_conts_string) {
            Ok(rep) => rep,
            Err(e) => return Err(DFError::SpecFileError),
        };

        let lib_name = mod_json_rep.lib.clone();

        // convert the api_info into module functions (missing signatures until discovery)
        let fns: HashMap<String, ModuleFunction> = mod_json_rep
            .fns
            .iter()
            .map(|(name, mod_fct_api)| (name.clone(), ModuleFunction::try_from(mod_fct_api)))
            .filter(|(_name, opt_mod_fct)| matches!(opt_mod_fct, Ok(_)))
            .map(|(name, opt_mod_fct)| (name, opt_mod_fct.unwrap()))
            .collect();
        Ok(Self {
            lib: lib_name,
            fns: fns,
        })
    }

    /// get the variable name corresponding to this module when it's imported in generated tests
    /// it's just the name of this module, switching hyphens to underscores
    pub fn get_mod_js_var_name(&self) -> String {
        str::replace(&self.lib, "-", "_").to_string()
    }

    /// return JS code to import this module
    pub fn get_js_for_basic_cjs_import(&self) -> String {
        [
            "let ",
            &self.get_mod_js_var_name(),
            " = require(\"",
            &self.lib,
            "\");",
        ]
        .join("")
    }

    pub fn short_display(&self) -> String {
        let mut to_print = serde_json::json!({"lib": self.lib});
        let mut sigs = serde_json::json!({});
        for (fc_name, fc_obj) in self.fns.clone() {
            let mut fn_sigs = vec![];
            let fn_name = fc_name.clone();
            for sig in fc_obj.get_sigs() {
                let args: Vec<String> = sig
                    .get_arg_list()
                    .iter()
                    .map(|arg| arg.get_type().to_string())
                    .collect();
                fn_sigs.push(serde_json::json!({"args": args.join(", "), "callback_res": sig.get_callback_res()}));
            }
            sigs[fn_name] = serde_json::json!(fn_sigs);
        }
        to_print["sigs"] = sigs;

        serde_json::to_string_pretty(&to_print).unwrap()
    }
}

/// representation of a function in a given module
/// each function has a list of valid signatures
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct ModuleFunction {
    /// name of the function
    name: String,
    /// list of valid signatures
    sigs: Vec<FunctionSignature>,
    /// number of arguments according to the API docs
    num_api_args: Option<usize>,
}

impl ModuleFunction {
    /// getter for num_api_args
    pub fn get_num_api_args(&self) -> Option<usize> {
        self.num_api_args
    }

    /// getter for signatures
    pub fn get_sigs(&self) -> &Vec<FunctionSignature> {
        &self.sigs
    }

    /// add a signature to the list of signatures
    pub fn add_sig(&mut self, sig: FunctionSignature) {
        self.sigs.push(sig);
    }

    // getter for name
    pub fn get_name(&self) -> String {
        self.name.clone()
    }
}

/// convert ModFctAPIJSON into a modulefunction
impl TryFrom<&ModFctAPIJSON> for ModuleFunction {
    type Error = DFError;

    fn try_from(mod_fct_api: &ModFctAPIJSON) -> Result<Self, Self::Error> {
        Ok(Self {
            name: mod_fct_api.name.clone(),
            sigs: Vec::new(),
            num_api_args: match mod_fct_api.used_default_args {
                Some(true) => None,
                _ => Some(mod_fct_api.num_args),
            },
        })
    }
}

/// representation of a single signature of a module function
/// this includes the number and types of arguments, etc
/// note that functions may have multiple valid signatures
#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub struct FunctionSignature {
    /// number of arguments
    num_args: usize,
    /// list of arguments: their type, and value if tested
    arg_list: Vec<FunctionArgument>,
    /// callback related result of running this test, if it was run
    call_test_result: Option<FunctionCallResult>,
}

impl TryFrom<(&Vec<FunctionArgument>, FunctionCallResult)> for FunctionSignature {
    type Error = DFError;

    fn try_from(
        (arg_list, callback_res): (&Vec<FunctionArgument>, FunctionCallResult),
    ) -> Result<Self, Self::Error> {
        let num_args = arg_list.len();
        Ok(Self {
            num_args,
            arg_list: arg_list.clone(),
            call_test_result: Some(callback_res),
        })
    }
}

impl FunctionSignature {
    /// constructor
    pub fn new(
        num_args: usize,
        arg_list: Vec<FunctionArgument>,
        call_test_result: Option<FunctionCallResult>,
    ) -> Self {
        Self {
            num_args,
            arg_list,
            call_test_result,
        }
    }

    /// get the positions of callback arguments for this function
    pub fn get_callback_positions(&self) -> Vec<usize> {
        let mut posns = Vec::new();
        for (pos, arg) in self.arg_list.iter().enumerate() {
            if arg.is_callback {
                posns.push(pos);
            }
        }
        posns
    }

    /// getter for arg list
    pub fn get_arg_list(&self) -> &Vec<FunctionArgument> {
        &self.arg_list
    }

    /// mutable getter for the arg list
    pub fn get_mut_args(&mut self) -> &mut Vec<FunctionArgument> {
        &mut self.arg_list
    }

    /// getter for callback res
    pub fn get_callback_res(&self) -> Option<FunctionCallResult> {
        self.call_test_result
    }
}

/// representation of a function argument
#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub struct FunctionArgument {
    /// type of the argumnet
    arg_type: ArgType,
    /// is this argument a callback? true/false
    is_callback: bool,
    // if tested, list of values tested with
    // TODO figure out how to represent these values
    string_rep_arg_val: Option<String>,
}

impl FunctionArgument {
    pub fn new(arg_type: ArgType, is_callback: bool, string_rep_arg_val: Option<String>) -> Self {
        if (arg_type == ArgType::CallbackType) != is_callback {
            panic!("If the FunctionArgument is a CallbackType it must also be a callback bool");
        }
        Self {
            arg_type,
            is_callback,
            string_rep_arg_val,
        }
    }
    /// getter for string representation of argument value
    pub fn get_string_rep_arg_val(&self) -> &Option<String> {
        &self.string_rep_arg_val
    }

    pub fn set_string_rep_arg_val(&mut self, rep_arg_val: String) {
        self.string_rep_arg_val = Some(rep_arg_val.clone())
    }

    pub fn get_type(&self) -> ArgType {
        self.arg_type
    }
}

/// list of types being tracked, for arguments
/// this can be modified for an arbitrary amount of granularity
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum ArgType {
    /// number
    NumberType,
    /// string
    StringType,
    /// array type
    ArrayType,
    /// non-callback, non-array, object
    ObjectType,
    /// callback (TODO maybe more granularity here)
    CallbackType,
    /// the "any" dynamic type (basically a no-op)
    AnyType,
}

impl std::fmt::Display for ArgType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            ArgType::NumberType => write!(f, "num"),
            ArgType::StringType => write!(f, "string"),
            ArgType::ArrayType => write!(f, "array"),
            ArgType::ObjectType => write!(f, "object"),
            ArgType::CallbackType => write!(f, "function"),
            ArgType::AnyType => write!(f, "any"),
        }
    }
}

/// errors in the DF testgen pipeline
#[derive(Debug)]
pub enum DFError {
    /// error reading some sort of spec file from a previous stage of the pipeline
    SpecFileError,
    /// error printing test file
    WritingTestError,
    /// error running test
    TestRunningError,
    /// error parsing test output
    TestOutputParseError,
    /// invalid test extension option
    InvalidTestExtensionOption,
}

/// representation of the different test outcomes we care about
/// in this case, the only test is only about the callback arguments (whether or not
/// they were called, and in what order)
#[derive(Debug, PartialEq, Eq, Clone, Copy, Deserialize, Serialize)]
pub enum SingleCallCallbackTestResult {
    /// callback is called and executed synchronously, and no error
    CallbackCalledSync,
    /// callback is called and executed asynchronously, and no error
    CallbackCalledAsync,
    /// callback is not called, and no error
    NoCallbackCalled,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Deserialize, Serialize)]
pub enum FunctionCallResult {
    SingleCallback(SingleCallCallbackTestResult),
    /// there is an error in the execution of the function
    ExecutionError,
    // TODO MultiCallback
}
