//! The data structures representing a JavaScript module.

use std::collections::{HashMap, HashSet};
use std::convert::TryFrom;
use std::path::PathBuf;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::consts::DEFAULT_MAX_ARG_LENGTH;
use crate::errors::*;
use crate::functions::*;
use crate::tests::{ExtensionPointID, Test};

/// Serializable representation of the module,
/// at the `api_info` stage (i.e., only statically looked at the properties of
/// importing the library).
#[derive(Debug, Serialize, Deserialize)]
struct NpmModuleJSON {
    /// Name of the module.
    lib: String,
    /// Map of functions making up the module,
    /// indexed by the name of the function.
    /// Here the functions are the output of the `api_info` phase
    /// optional string in the hashmap index is the access path of the fct receiver
    fns: HashMap<String, ModFctAPIJSON>,
}

/// Serializable representation of the function as discovered by the `api_info`.
/// This is just static info from the API.
#[derive(Debug, Serialize, Deserialize)]
struct ModFctAPIJSON {
    /// Name of the function.
    name: String,
    /// Number of arguments.
    num_args: usize,
    /// Indicator of whether or not the API function has a specified
    /// number of args.
    used_default_args: Option<bool>,
    // Signatures
    sigs: Vec<FunctionSignature>,
}

/// Module class:
/// - represents the library
/// - composed of a list of functions
/// - each function is composed of a list of signatures
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct NpmModule {
    /// Name of the npm module.
    pub(crate) lib: String,
    /// Optional custom import code for the module.
    pub(crate) import_code: Option<String>,
    /// Map of functions making up the module,
    /// indexed by the name of the function
    fns: HashMap<(AccessPathModuleCentred, String), ModuleFunction>,
}

/// Automatically cast from NpmModule back to NpmModuleJSON (for printing to/reading from files)
impl From<&NpmModule> for NpmModuleJSON {
    fn from(mod_rep: &NpmModule) -> Self {
        Self {
            lib: mod_rep.lib.clone(),
            fns: mod_rep
                .get_fns()
                .iter()
                .map(|((acc_path, name), mod_fct)| {
                    ([name, ", ", &acc_path.to_string()].join(""), mod_fct.into())
                })
                .collect::<HashMap<String, ModFctAPIJSON>>(),
        }
    }
}

/// Pretty printing for the NpmModule (JSON style).
impl std::fmt::Debug for NpmModule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match serde_json::to_string_pretty(&NpmModuleJSON::from(self)) {
            Ok(pretty_json) => write!(f, "{}", pretty_json),
            Err(_) => Err(std::fmt::Error),
        }
    }
}

impl NpmModule {
    /// Setter for the list of functions in the module.
    pub fn set_fns(
        &mut self,
        new_fcts: HashMap<(AccessPathModuleCentred, String), ModuleFunction>,
    ) {
        self.fns = new_fcts;
    }

    /// Getter for the module functions.
    pub fn get_fns(&self) -> &HashMap<(AccessPathModuleCentred, String), ModuleFunction> {
        &self.fns
    }

    /// Mutable getter for the module functions.
    pub fn get_mut_fns(
        &mut self,
    ) -> &mut HashMap<(AccessPathModuleCentred, String), ModuleFunction> {
        &mut self.fns
    }

    pub fn add_fcts_rooted_in_ret_vals(
        &mut self,
        accpath_fct_props: &HashMap<AccessPathModuleCentred, Vec<String>>,
    ) {
        // iterate through all the new functions
        // add them as empty `ModuleFunction`s to the module function list
        let fns = self.get_mut_fns();
        for (accpath, fct_prop_names) in accpath_fct_props.iter() {
            for name in fct_prop_names.iter() {
                fns.insert(
                    (accpath.clone(), name.to_string()),
                    ModuleFunction {
                        name: name.to_string(),
                        sigs: HashSet::new(),
                        // some heuristics: `then` and `catch` methods (on Promises) take 1 arg
                        num_api_args: match name.as_str() {
                            "then" | "catch" => Some(1),
                            _ => None,
                        },
                    },
                );
            }
        }
    }

    pub fn add_function_sigs_from_test(
        &mut self,
        test: &Test,
        ext_point_results: &HashMap<ExtensionPointID, (FunctionCallResult, Option<String>)>,
    ) {
        for (ext_point_id, (fct_result, _)) in ext_point_results.iter() {
            let rel_fct = test.get_fct_call_from_id(ext_point_id);
            if let Some(rel_fct) = rel_fct && fct_result != &FunctionCallResult::ExecutionError {
                let fct_name = rel_fct.get_name();
                let base_mod_import = AccessPathModuleCentred::RootPath(self.lib.clone());
                let fct_acc_path_rep: AccessPathModuleCentred =
                    match rel_fct.get_acc_path() {
                        Some(ap) => ap,
                        None => &base_mod_import,
                    }.clone();
                let mut new_sig = rel_fct.sig.clone();
                new_sig.set_call_res(*fct_result);
                if let Some(mut_fct_desc) = self.fns.get_mut(&(
                    (fct_acc_path_rep).clone().get_base_path().unwrap_or_else(|| {
                        &base_mod_import
                    }).clone(),
                    fct_name.to_string(),
                )) {
                    mut_fct_desc.add_sig(new_sig.clone());
                }
            }
        }
    }

    /// Create an `NpmModule` object from a JSON file resulting from running the `api_info`
    /// phase: this is just a list of all the functions for a module, without having
    /// run the discovery phase yet (i.e., no arg info yet).
    pub fn from_api_spec(
        path: PathBuf,
        _mod_name: String,
        import_code_file: Option<PathBuf>,
    ) -> Result<Self, DFError> {
        let file_conts = std::fs::read_to_string(path);
        let file_conts_string = match file_conts {
            Ok(fcs) => fcs,
            _ => return Err(DFError::SpecFileError),
        };

        let import_code = match import_code_file {
            Some(filename) => match std::fs::read_to_string(filename) {
                Ok(conts) => Some(conts),
                _ => return Err(DFError::SpecFileError),
            },
            None => None,
        };

        let mod_json_rep: NpmModuleJSON = match serde_json::from_str(&file_conts_string) {
            Ok(rep) => rep,
            Err(_) => return Err(DFError::SpecFileError),
        };

        let lib_name = mod_json_rep.lib.clone();

        // convert the api_info into module functions (missing signatures until discovery)
        let fns: HashMap<(AccessPathModuleCentred, String), ModuleFunction> = mod_json_rep
            .fns
            .iter()
            .map(|(name_and_opt_path, mod_fct_api)| {
                let mut name_path_iter = name_and_opt_path.split(", ");
                let name = name_path_iter.next().unwrap();
                let opt_rec_acc_path_string = name_path_iter.next();
                (
                    (
                        match opt_rec_acc_path_string {
                            Some(acc) => {
                                AccessPathModuleCentred::from_str(acc).unwrap_or_else(|_| {
                                    AccessPathModuleCentred::RootPath(lib_name.clone())
                                })
                            }
                            _ => AccessPathModuleCentred::RootPath(lib_name.clone()),
                        },
                        name.to_string(),
                    ),
                    ModuleFunction::try_from(mod_fct_api),
                )
            })
            .filter(|(_name_and_path, opt_mod_fct)| matches!(opt_mod_fct, Ok(_)))
            .map(|(name_and_path, opt_mod_fct)| (name_and_path, opt_mod_fct.unwrap()))
            .collect();
        Ok(Self {
            lib: lib_name,
            fns,
            import_code,
        })
    }

    /// Get the variable name corresponding to this module when it's imported in generated tests
    /// (it's just the name of this module, switching hyphens to underscores).
    pub fn get_mod_js_var_name(&self) -> String {
        str::replace(&self.lib, "-", "_")
    }

    /// Short string representation of the module, mainly for debugging/display.
    pub fn short_display(&self) -> String {
        let mut to_print = serde_json::json!({"lib": self.lib});
        let mut sigs = serde_json::json!({});
        for ((acc_path_and_sig, _), fc_obj) in self.fns.clone() {
            let mut fn_sigs = vec![];
            let acc_path_and_sig = acc_path_and_sig.clone();
            for sig in fc_obj.get_sigs() {
                let args: Vec<String> = sig
                    .get_arg_list()
                    .iter()
                    .map(|arg| arg.get_type().to_string())
                    .collect();
                fn_sigs.push(serde_json::json!({"args": args.join(", "), "callback_res": sig.get_call_res()}));
            }
            sigs[acc_path_and_sig.to_string()] = serde_json::json!(fn_sigs);
        }
        to_print["sigs"] = sigs;

        serde_json::to_string_pretty(&to_print).unwrap()
    }
}

/// Representation of a function in a given module;
/// each function has a list of valid signatures
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct ModuleFunction {
    /// Name of the function.
    name: String,
    /// List of valid signatures.
    sigs: HashSet<FunctionSignature>,
    /// Number of arguments according to the API docs (`None` if
    /// this info couldn't be found, e.g., if the signature has
    /// the spread args).
    num_api_args: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub enum FieldNameType {
    StringField(String),
    IndexField(usize),
}
type ParamIndexType = usize;

/// Representation of access paths, rooted in a module import
/// (APs are defined in a bunch of papers including
/// [ours](https://drops.dagstuhl.de/opus/volltexte/2021/14029/pdf/DARTS-7-2-5.pdf))
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub enum AccessPathModuleCentred {
    /// Base case: the module import, with the module name
    RootPath(String),
    /// Recursive cases:
    /// Return of calling a function represented by another access path
    ReturnPath(Box<AccessPathModuleCentred>),
    /// Accessing a field (with specified name/index) represented by another access path
    FieldAccPath(Box<AccessPathModuleCentred>, FieldNameType),
    /// A parameter (of specified index) of a function call represented by another access path
    ParamPath(Box<AccessPathModuleCentred>, ParamIndexType),
    /// A new instance of a constructor represented by another access path (i.e., `new SomeClassFromModule()`)
    InstancePath(Box<AccessPathModuleCentred>),
}

impl AccessPathModuleCentred {
    /// Get the base path of the access path (removing the outer recursive level).
    /// Eg. `fs.readFile` has base path `fs`.
    /// Module import roots have no base path.
    pub fn get_base_path(&self) -> Option<&Self> {
        match self {
            Self::RootPath(_) => None,
            Self::ReturnPath(ret)
            | Self::FieldAccPath(ret, _)
            | Self::ParamPath(ret, _)
            | Self::InstancePath(ret) => Some(ret),
        }
    }
}

/// Autocast from strings to access paths
impl std::str::FromStr for AccessPathModuleCentred {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // delete all the characters resulting from printing the JSON rep
        let s = s
            .to_string()
            .replace("(\"", "")
            .replace("\")", "")
            .replace("StringField", "")
            .replace("IndexField", "");
        if s.ends_with(')') {
            // let s = s.split(")").next().ok_or(())?;
            let s = s[0..s.len() - 1].to_string();
            if s.starts_with("(module ") {
                let mut iter = s.split("(module ");
                iter.next(); // empty string is first
                return Ok(AccessPathModuleCentred::RootPath(
                    iter.next().ok_or(())?.to_string(),
                ));
            }
            // other base case: report AP as module_name.exports.<member>
            else if s.starts_with("(member exports (module ") {
                let s = s[0..s.len() - 1].to_string(); // cut off the extra closing paren in this double-case
                let mut iter = s.split("(member exports (module ");
                iter.next(); // empty string is first
                return Ok(AccessPathModuleCentred::RootPath(
                    iter.next().ok_or(())?.to_string(),
                ));
            } else if s.starts_with("(return ") {
                let mut iter = s.split("(return ");
                iter.next(); // empty string is first
                             // get the rest of the path
                let return_path = iter.intersperse("(return ").collect::<String>();
                return Ok(AccessPathModuleCentred::ReturnPath(Box::new(
                    AccessPathModuleCentred::from_str(&return_path)?,
                )));
            } else if s.starts_with("(member ") {
                let mut member_iter = s.split(' ');
                member_iter.next(); // first string is just "(member"
                let member_name = member_iter.next().ok_or(())?;
                let member_name = match member_name.parse::<usize>() {
                    Ok(val) => FieldNameType::IndexField(val),
                    _ => FieldNameType::StringField(member_name.to_string()),
                };
                // collect the rest of the iterator
                let member_path = member_iter.intersperse(" ").collect::<String>();
                return Ok(AccessPathModuleCentred::FieldAccPath(
                    Box::new(AccessPathModuleCentred::from_str(&member_path)?),
                    member_name,
                ));
            } else if s.starts_with("(parameter ") {
                let mut param_iter = s.split(' ');
                param_iter.next(); // first string is just "(param"
                let param_val = match param_iter.next().ok_or(())?.parse::<ParamIndexType>() {
                    Ok(val) => val,
                    _ => {
                        return Err(());
                    }
                };
                let param_path = param_iter.intersperse(" ").collect::<String>();
                return Ok(AccessPathModuleCentred::ParamPath(
                    Box::new(AccessPathModuleCentred::from_str(&param_path)?),
                    param_val,
                ));
            } else if s.starts_with("(new ") {
                let mut iter = s.split("(new ");
                iter.next(); // empty string is first
                             // collect the rest of the path
                let new_path = iter.intersperse("(new ").collect::<String>();
                return Ok(AccessPathModuleCentred::InstancePath(Box::new(
                    AccessPathModuleCentred::from_str(&new_path)?,
                )));
            }
        }
        Err(())
    }
}

impl std::fmt::Display for AccessPathModuleCentred {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::RootPath(mod_name) => write!(f, "(module {})", mod_name),
            Self::ReturnPath(rec_ap_box) => write!(f, "(return {})", *rec_ap_box),
            Self::FieldAccPath(rec_ap_box, field_name) => write!(
                f,
                "({})",
                format!("member {:?} {}", field_name, *rec_ap_box)
            ),
            Self::ParamPath(rec_ap_box, param_index) => {
                write!(f, "({})", format!("param {} {}", param_index, *rec_ap_box))
            }
            Self::InstancePath(rec_ap_box) => write!(f, "(new {})", *rec_ap_box),
        }
    }
}

impl ModuleFunction {
    /// Getter for `num_api_args`.
    pub fn get_num_api_args(&self) -> Option<usize> {
        self.num_api_args
    }

    /// Getter for signatures of this function.
    pub fn get_sigs(&self) -> &HashSet<FunctionSignature> {
        &self.sigs
    }

    /// Add a signature to the list of signatures of this function.
    pub fn add_sig(&mut self, sig: FunctionSignature) {
        self.sigs.insert(sig);
    }

    // Getter for function name.
    pub fn get_name(&self) -> String {
        self.name.clone()
    }
}

/// Convert `ModFctAPIJSON` into a `ModuleFunction`.
impl TryFrom<&ModFctAPIJSON> for ModuleFunction {
    type Error = DFError;

    fn try_from(mod_fct_api: &ModFctAPIJSON) -> Result<Self, Self::Error> {
        Ok(Self {
            name: mod_fct_api.name.clone(),
            sigs: HashSet::new(),
            num_api_args: match mod_fct_api.used_default_args {
                Some(true) => None,
                _ => Some(mod_fct_api.num_args),
            },
        })
    }
}

/// Convert `ModuleFunction` into a `ModFctAPIJSON`.
impl From<&ModuleFunction> for ModFctAPIJSON {
    fn from(mod_fct: &ModuleFunction) -> Self {
        let (num_args, used_default_args) = match mod_fct.num_api_args {
            Some(num_args) => (num_args, Some(false)),
            None => (DEFAULT_MAX_ARG_LENGTH, Some(true)),
        };
        Self {
            name: mod_fct.name.clone(),
            num_args,
            used_default_args,
            sigs: mod_fct
                .sigs
                .iter()
                .cloned()
                .collect::<Vec<FunctionSignature>>(),
        }
    }
}
