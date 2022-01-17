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
    num_args: i32,
    /// indicator of whether or not the API function has a specified
    /// number of args
    used_default_args: Option<bool>,
}

/// Module class:
/// - represents the library
/// - composed of a list of functions
/// - each function is composed of a list of signatures
#[derive(Serialize, Deserialize)]
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
            Err(_) => return Err(DFError::SpecFileError),
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
}

/// representation of a function in a given module
/// each function has a list of valid signatures
#[derive(Debug, Serialize, Deserialize)]
pub struct ModuleFunction {
    /// name of the function
    name: String,
    /// list of valid signatures
    sigs: Vec<FunctionSignature>,
    /// number of arguments according to the API docs
    num_api_args: Option<i32>,
}

impl ModuleFunction {
    /// getter for num_api_args
    pub fn get_num_api_args(&self) -> Option<i32> {
        self.num_api_args
    }

    /// getter for signatures
    pub fn get_sigs(&self) -> &Vec<FunctionSignature> {
        &self.sigs
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
#[derive(Debug, Serialize, Deserialize)]
pub struct FunctionSignature {
    /// number of arguments
    num_args: i32,
    /// is it async? true/false
    is_async: bool,
    /// list of arguments: their type, and value if tested
    arg_list: Vec<FunctionArgument>,
}

impl FunctionSignature {
    /// constructor
    pub fn new(num_args: i32, is_async: bool, arg_list: Vec<FunctionArgument>) -> Self {
        Self {
            num_args,
            is_async,
            arg_list
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
}

/// representation of a function argument
#[derive(Debug, Serialize, Deserialize)]
pub struct FunctionArgument {
    /// type of the argumnet
    arg_type: ArgType,
    /// is this argument a callback? true/false
    is_callback: bool,
    // if tested, list of values tested with
    // TODO figure out how to represent these values
    string_rep_arg_val: String,
}

impl FunctionArgument {
    pub fn new(arg_type: ArgType, is_callback: bool, string_rep_arg_val: String) -> Self {
        Self {
            arg_type,
            is_callback,
            string_rep_arg_val,
        }
    }
    /// getter for string representation of argument value
    pub fn get_string_rep_arg_val(&self) -> &String {
        &self.string_rep_arg_val
    }
}

/// list of types being tracked, for arguments
/// this can be modified for an arbitrary amount of granularity
#[derive(Debug, Serialize, Deserialize)]
pub enum ArgType {
    /// the "any" dynamic type (basically a no-op)
    AnyType,
    /// number
    NumberType,
    /// string
    StringType,
    /// callback (TODO maybe more granularity here)
    CallbackType,
    /// array type
    ArrayType,
    /// non-callback, non-array, object
    ObjectType,
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
}
