/// the data structures representing a module
use std::collections::HashMap;
use std::convert::TryFrom;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::errors::*;
use crate::functions::*;

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
    pub fn get_js_for_basic_cjs_import(&self, api_src_dir: Option<String>) -> String {
        [
            "let ",
            &self.get_mod_js_var_name(),
            " = require(\"",
            &match api_src_dir {
                Some(dir) => dir,
                None => self.lib.clone(),
            },
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
                fn_sigs.push(serde_json::json!({"args": args.join(", "), "callback_res": sig.get_call_res()}));
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
