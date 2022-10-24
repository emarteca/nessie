/// the data structures representing a module
use std::collections::HashMap;
use std::convert::TryFrom;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::testgen::{Callback, ExtensionType};

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
    pub fn set_fns(&mut self, new_fcts: HashMap<String, ModuleFunction>) {
        self.fns = new_fcts;
    }

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
    /// list of arguments: their type, and value if tested
    arg_list: Vec<FunctionArgument>,
    /// callback related result of running this test, if it was run
    call_test_result: Option<FunctionCallResult>,
    /// is spread (i.e., `...args`)
    #[serde(default)] // default is false
    pub is_spread_args: bool,
}

impl TryFrom<(&Vec<FunctionArgument>, FunctionCallResult)> for FunctionSignature {
    type Error = DFError;

    fn try_from(
        (arg_list, callback_res): (&Vec<FunctionArgument>, FunctionCallResult),
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            arg_list: arg_list.clone(),
            call_test_result: Some(callback_res),
            is_spread_args: false,
        })
    }
}

impl FunctionSignature {
    /// constructor
    pub fn new(
        arg_list: &Vec<FunctionArgument>,
        call_test_result: Option<FunctionCallResult>,
    ) -> Self {
        Self {
            arg_list: arg_list.clone(),
            call_test_result,
            is_spread_args: false,
        }
    }

    /// get the positions of callback arguments for this function
    pub fn get_callback_positions(&self) -> Vec<usize> {
        let mut posns = Vec::new();
        for (pos, arg) in self.arg_list.iter().enumerate() {
            if arg.is_callback() {
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

impl std::default::Default for FunctionSignature {
    fn default() -> Self {
        Self {
            arg_list: Vec::new(),
            call_test_result: None,
            is_spread_args: true,
        }
    }
}

/// representation of a function argument
#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub struct FunctionArgument {
    /// type of the argument
    arg_type: ArgType,
    // if tested, list of values tested with
    // TODO figure out how to represent these values
    arg_val: Option<ArgVal>,
}

impl FunctionArgument {
    pub fn new(arg_type: ArgType, arg_val: Option<ArgVal>) -> Self {
        Self { arg_type, arg_val }
    }

    pub fn is_callback(&self) -> bool {
        self.arg_type == ArgType::CallbackType
    }

    /// getter for string representation of argument value
    pub fn get_string_rep_arg_val(&self, extra_body_code: Option<String>) -> Option<String> {
        Some(
            self.arg_val
                .clone()?
                .get_string_rep(extra_body_code)
                .clone(),
        )
    }

    pub fn get_string_rep_arg_val__short(&self) -> Option<String> {
        match self.arg_type {
            ArgType::CallbackType => Some("\"[function]\"".to_string()),
            _ => self.get_string_rep_arg_val(None).clone(),
        }
    }

    pub fn set_arg_val(&mut self, arg_val: ArgVal) -> Result<(), TestGenError> {
        if !(arg_val.get_type().can_be_repd_as(self.arg_type)) {
            return Err(TestGenError::ArgTypeValMismatch);
        }
        self.arg_val = Some(arg_val.clone());
        Ok(())
    }

    pub fn get_arg_val(&self) -> &Option<ArgVal> {
        &self.arg_val
    }

    pub fn get_arg_val_mut(&mut self) -> &Option<ArgVal> {
        &mut self.arg_val
    }

    pub fn get_type(&self) -> ArgType {
        self.arg_type
    }

    pub fn set_cb_id(&mut self, cb_id: Option<String>) -> Result<(), TestGenError> {
        match self.arg_val.as_mut() {
            Some(mut arg_val) => {
                arg_val.set_cb_id(cb_id)?;
                Ok(())
            }
            _ => Err(TestGenError::ArgValNotSetYet),
        }
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

impl ArgType {
    // return true if the receiver (`self`) can be represented
    // by the other type `ot`.
    pub fn can_be_repd_as(&self, ot: Self) -> bool {
        *self == ot || ot == Self::AnyType
    }
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArgVal {
    Number(String),
    String(String),
    Array(String),
    Object(String),
    Callback(CallbackVal),
}

impl ArgVal {
    pub fn get_string_rep(&self, extra_body_code: Option<String>) -> String {
        match self {
            Self::Number(s) | Self::String(s) | Self::Array(s) | Self::Object(s) => s.clone(),
            Self::Callback(cbv) => cbv.get_string_rep(extra_body_code),
        }
    }

    pub fn get_type(&self) -> ArgType {
        match self {
            Self::Number(_) => ArgType::NumberType,
            Self::String(_) => ArgType::StringType,
            Self::Array(_) => ArgType::ArrayType,
            Self::Object(_) => ArgType::ObjectType,
            Self::Callback(_) => ArgType::CallbackType,
        }
    }

    pub fn set_cb_id(&mut self, cb_id: Option<String>) -> Result<(), TestGenError> {
        match self {
            Self::Callback(CallbackVal::RawCallback(cb)) => {
                cb.set_cb_id(cb_id);
                Ok(())
            }
            _ => Err(TestGenError::ArgTypeValMismatch),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CallbackVal {
    Var(String),
    RawCallback(Callback),
}

impl CallbackVal {
    pub fn get_string_rep(&self, extra_body_code: Option<String>) -> String {
        match self {
            Self::Var(vs) => vs.clone(),
            Self::RawCallback(cb) => cb.get_string_rep(extra_body_code),
        }
    }
}

/// errors in the DF testgen pipeline
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum DFError {
    /// error reading some sort of spec file from a previous stage of the pipeline
    SpecFileError,
    /// error printing test file
    WritingTestError,
    /// error running test (could be a timeout)
    TestRunningError,
    /// error parsing test output
    TestOutputParseError,
    /// invalid test extension option
    InvalidTestExtensionOption,
    /// error during test generation
    TestGenError(TestGenError),
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum TestGenError {
    /// type mismatch between arg value and specified arg type
    ArgTypeValMismatch,
    /// trying to set a property of an arg val that is still None
    ArgValNotSetYet,
}

impl From<TestGenError> for DFError {
    fn from(tge: TestGenError) -> Self {
        Self::TestGenError(tge)
    }
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

impl FunctionCallResult {
    pub fn can_be_extended(&self, ext_type: ExtensionType) -> bool {
        match (self, ext_type) {
            // can never extend if there's an execution error
            (Self::ExecutionError, _) => false,
            // can't nest if there's no callback
            (
                Self::SingleCallback(SingleCallCallbackTestResult::NoCallbackCalled),
                ExtensionType::Nested,
            ) => false,
            // no-callback and sequential: true
            // sync or async callback and either nested or sequential: true
            (_, _) => true,
        }
    }
}
