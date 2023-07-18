//! Representation and use of mined data (used as a seed for the test generator).
//!
//! TODO: in the improved version of the test generator, we're going to mine
//! much more information -- the current struct representing a mined data point
//! only represents nesting relationships.
//! This is going to get totally overhauled.

use crate::errors::*;
use crate::functions::{ArgType, ArgVal, FunctionArgument, FunctionSignature};
use crate::module_reps::{AccessPathModuleCentred, FieldNameType};
use crate::tests::FunctionCall;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::convert::TryFrom;
use std::path::PathBuf;
use std::str::FromStr;

/*
    Example of what these pairs look like right now:

    {
        "outer_pkg": "\"fs\"",
        "outer_fct": "realpath",
        "outer_params": [
            {
                "ident": "outer_arg_0"
            },
            {
                "ident": "outer_arg_1"
            },
            {
                "callback": "CALLBACK"
            }
        ],
        "inner_pkg": "\"q\"",
        "inner_fct": "reject",
        "inner_params": [
            {
                "object": "OBJECT"
            },
            {
                "ident": "outer_arg_0"
            }
        ]
    },
*/
/// Representation of a mined nesting pair.
/// Currently the only information represented is the package and names of the
/// functions in the nesting, limited information on the types of the arguments,
/// and any dataflow between other arguments to the outer function, and arguments
/// to the inner call (nested in the callback).
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MinedNestingPairJSON {
    /// Name of the module the outer function belongs to.
    outer_pkg: String,
    /// Name of the outer function.
    outer_fct: String,
    /// Arguments to the outer function.
    outer_params: Vec<MinedParam>,
    /// Name of the module the inner function belongs to.
    inner_pkg: String,
    /// Name of the inner function.
    inner_fct: String,
    /// Arguments to the inner function.
    inner_params: Vec<MinedParam>,
}

/// Database of mined data, indexed by the library associated with the outer function in the nested pair.
pub type LibMinedData = HashMap<String, Vec<MinedNestingPairJSON>>;

/// Database of mined call data, indexed by the library associated with the function being called.
pub type LibMinedCallData = HashMap<String, Vec<MinedAPICall>>;

impl MinedNestingPairJSON {
    /// Read a file (output from the data mining), that has a list of JSON representations
    /// of mined nesting pairs.
    /// Return the corresponding list, or an error if the file is malformed.
    pub fn list_from_file(path: &PathBuf) -> Result<Vec<Self>, DFError> {
        let file_conts = std::fs::read_to_string(path);
        let file_conts_string = match file_conts {
            Ok(fcs) => fcs,
            _ => return Err(DFError::MinedDataFileError),
        };

        let mined_data_rep: Vec<Self> = match serde_json::from_str(&file_conts_string) {
            Ok(rep) => rep,
            Err(_) => return Err(DFError::MinedDataFileError),
        };

        Ok(mined_data_rep)
    }

    /// Turn a list of mined nesting pairs into a map of lists indexed by the library
    /// the outer function originates from.
    pub fn lib_map_from_list(all_pairs: Vec<Self>) -> LibMinedData {
        let mut ret_map = HashMap::new();
        for pair in all_pairs {
            ret_map
                .entry(pair.get_outer_pkg())
                .or_insert(Vec::new())
                .push(pair);
        }
        ret_map
    }

    /// Getter for the outer package name.
    pub fn get_outer_pkg(&self) -> String {
        // in the mined data, the package name is surrounded by "", so strip these
        self.outer_pkg.replace('\"', "")
    }

    /// Getter for the inner package name.
    pub fn get_inner_pkg(&self) -> String {
        // in the mined data, the package name is surrounded by "", so strip these
        self.inner_pkg.replace('\"', "")
    }

    // get position of the first callback in the argument list of the outer function
    pub fn get_outer_first_cb_arg_position(&self) -> Option<usize> {
        for (pos, arg) in self.outer_params.iter().enumerate() {
            if arg.is_callback() {
                return Some(pos);
            }
        }
        None
    }
}

/// Serializable representation of an argument in a mined function nesting
/// (both outer and inner function arguments).
///
/// TODO: when we redo the data mining, the structure of the params will be improved
/// and have more information.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MinedParam {
    // the param is either an ident, a callback, or an object
    // if callback, the string is just "CALLBACK"
    // if object, the string is just "OBJECT
    // if ident, then it's the name of the parameter: either "outer_arg_" or "inner_arg_" followed by
    // the argument position.
    // The important part here is that it represents dataflow between the args of the outer to inner functions
    /// Non-callback, non-object argument.
    ident: Option<String>,
    /// Callback argument.
    callback: Option<String>,
    /// Object argument.
    object: Option<String>,
}

impl MinedParam {
    /// Check if a parameter is valid: it must be either a callback, an object, or
    /// an ident (and only one at once).
    pub fn is_valid(&self) -> bool {
        matches!(
            (&self.ident, &self.callback, &self.object),
            (Some(_), None, None) | (None, Some(_), None) | (None, None, Some(_))
        )
    }

    /// Check if the parameter is a callback.
    pub fn is_callback(&self) -> bool {
        self.is_valid() && self.callback.is_some()
    }

    /// Get the argument type of the mined parameter.
    /// Returns an error if the parameter is invalid.
    /// TODO: right now the only mined types are callbacks, objects, and
    /// all the `ident` arguments are considered `any` typed since no other
    /// information was mined.
    /// In the improved mining analysis, we'll get more information about types
    /// for arguments with statically available values.
    pub fn get_arg_type(&self) -> Result<ArgType, DFError> {
        if !self.is_valid() {
            return Err(DFError::InvalidMinedData(self.to_string()));
        }
        // this is all the granularity we have in the mined data right now :'(
        Ok(if self.callback.is_some() {
            ArgType::CallbackType
        } else if self.object.is_some() {
            ArgType::ObjectType
        } else {
            ArgType::AnyType
        })
    }
}

impl std::fmt::Display for MinedParam {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "ident: {:?}, object: {:?}, callback: {:?}",
            self.ident, self.object, self.callback
        )
    }
}

/// Representation of a generated inner function call, as a nested extension based
/// on some mined data.
/// It includes the name and signature of the inner function, and a list of the dataflow from
/// arguments to the outer call to this inner call.
#[derive(Debug, Clone)]
pub struct MinedDataNestedExtension {
    /// Name of the inner function.
    pub fct_name: String,
    /// Signature of the inner function.
    pub sig: FunctionSignature,
    /// List of pairs of: position of argument in outer function call, passed to position in inner call.
    pub outer_to_inner_dataflow: Vec<(usize, usize)>,
}

impl TryFrom<&MinedParam> for FunctionArgument {
    type Error = DFError;

    fn try_from(mined_param: &MinedParam) -> Result<Self, Self::Error> {
        Ok(FunctionArgument::new(
            mined_param.get_arg_type()?,
            None, /* no arg val */
        ))
    }
}

impl TryFrom<&Vec<MinedParam>> for FunctionSignature {
    type Error = DFError;

    fn try_from(mined_params: &Vec<MinedParam>) -> Result<Self, Self::Error> {
        let mut arg_list: Vec<FunctionArgument> = Vec::with_capacity(mined_params.len());
        for param in mined_params {
            arg_list.push(FunctionArgument::try_from(param)?);
        }
        Ok(FunctionSignature::new(
            &arg_list, None, /* no call test result */
        ))
    }
}

/// Given a list of mined data pairs and an outer function call to extend,
/// return a list of all valid nested extensions from the mined data
/// (empty list if none are valid).
pub fn get_rel_mined_data_nested_extensions(
    outer_fct: Option<&FunctionCall>,
    pkg_name: &String,
    mined_data: &[MinedNestingPairJSON],
) -> Vec<MinedDataNestedExtension> {
    if outer_fct.is_none() {
        return Vec::new();
    }
    let outer_fct = outer_fct.unwrap();
    // can't nest if the outer function has no callback argument
    if !outer_fct.sig.has_cb_arg() {
        return Vec::new();
    }
    let outer_arg_len = outer_fct.sig.get_arg_list().len();
    let outer_fct_name = outer_fct.get_name();

    mined_data
        .iter()
        .filter_map(|mined_pair| {
            let inner_fct_sig = FunctionSignature::try_from(&mined_pair.inner_params);
            // note: right now we only support nestings from functions from the same package
            // for a nesting to be a valid for extending the `outer_fct`:
            // -- outer package matches origin package of function to be nested extended
            // -- inner package matches origin package of function to be nested extended
            // -- outer function matches the function being nested extended
            // -- outer function signature has compatible signature (i.e., same number of arguments)
            //    as the function being nested extended
            // -- inner function signature is properly parsed from the mined data
            if &mined_pair.get_outer_pkg() == pkg_name
                && &mined_pair.get_inner_pkg() == pkg_name
                && mined_pair.outer_fct == outer_fct_name
                && mined_pair.outer_params.len() == outer_arg_len
                && inner_fct_sig.is_ok()
            {
                let inner_fct_name = mined_pair.inner_fct.clone();
                let inner_fct_sig = inner_fct_sig.unwrap();

                let outer_to_inner_dataflow = mined_pair.inner_params
                    .iter()
                    .enumerate()
                    .filter_map(|(pos, inner_param)| {
                        if let Some(var_name) = &inner_param.ident && var_name.starts_with("outer_arg_") {
                            // get the string after the last _ and convert to a usize
                            let (_, outer_pos) = var_name.rsplit_once('_').unwrap();
                            Some((outer_pos.parse::<usize>().unwrap(), pos))
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<(usize, usize)>>();

                Some(MinedDataNestedExtension {
                    fct_name: inner_fct_name,
                    sig: inner_fct_sig,
                    outer_to_inner_dataflow,
                })
            } else {
                None
            }
        })
        .collect::<Vec<MinedDataNestedExtension>>()
}

/*
    Example mined data for single API call w/ at least one statically available argument.

    {
        "pkg": "path",
        "acc_path": "(member join (member exports (module path)))",
        "sig_with_types": "(_NOT_CONST_OR_FCT_,string)",
        "sig_with_values": "(_NOT_CONST_OR_FCT_,'jsonfile-tests-readfile-sync')"
    }
*/

/// Representation of a mined API call.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MinedAPICallJSON {
    /// Name of the module the function belongs to.
    pkg: String,
    /// Access path representation of the call.
    acc_path: String,
    /// Signature of the call, with types (as statically available)
    sig_with_types: String,
    /// Signature of the call, with values as statically available
    sig_with_values: String,
}

impl MinedAPICallJSON {
    /// Read a file (output from the data mining), that has a list of JSON representations
    /// of mined API calls.
    /// Return the corresponding list, or an error if the file is malformed.
    pub fn list_from_file(path: &PathBuf) -> Result<Vec<Self>, DFError> {
        let file_conts = std::fs::read_to_string(path);
        let file_conts_string = match file_conts {
            Ok(fcs) => fcs,
            _ => return Err(DFError::MinedDataFileError),
        };

        let mined_data_rep: Vec<Self> = match serde_json::from_str(&file_conts_string) {
            Ok(rep) => rep,
            Err(_) => {
                return Err(DFError::MinedDataFileError);
            }
        };

        Ok(mined_data_rep)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MinedAPICall {
    /// Name of the module the function belongs to.
    pkg: String,
    /// Access path representation of the call.
    acc_path: AccessPathModuleCentred,
    /// Signature of the call, with types (as statically available)
    sig_with_types: Vec<Option<ArgType>>,
    /// Signature of the call, with values as statically available
    sig_with_values: Vec<Option<ArgVal>>,
}

impl MinedAPICall {
    pub fn get_sig_with_types(&self) -> &Vec<Option<ArgType>> {
        &self.sig_with_types
    }

    pub fn get_sig_with_vals(&self) -> &Vec<Option<ArgVal>> {
        &self.sig_with_values
    }

    /// Read a file (output from the data mining), that has a list of JSON representations
    /// of mined API calls.
    /// Return the corresponding list, or an error if the file is malformed.
    pub fn list_from_file(path: &PathBuf) -> Result<Vec<Self>, DFError> {
        let json_vec = MinedAPICallJSON::list_from_file(path)?;

        let mut res: Vec<Self> = Vec::with_capacity(json_vec.len());
        for api_call in json_vec.into_iter() {
            let mut sig_with_types: Vec<Option<ArgType>> = Vec::new();
            let mut sig_with_values: Vec<Option<ArgVal>> = Vec::new();
            for (ty, val) in api_call
                .sig_with_types
                .split(',')
                .zip(api_call.sig_with_values.split(','))
            {
                let opt_ty = match ty {
                    "Object" => Some(ArgType::ObjectType),
                    "string" => Some(ArgType::StringType),
                    "bool" => Some(ArgType::BoolType),
                    "number" => Some(ArgType::NumberType),
                    "null" => Some(ArgType::NullType),
                    // TODO should we deal with regex or bigint?
                    "Array" => Some(ArgType::ArrayType),
                    "_FUNCTION_" => Some(ArgType::CallbackType),
                    _ => None,
                };
                let opt_val = match (val, opt_ty) {
                    (s, Some(ArgType::ObjectType)) => Some(ArgVal::Object(s.to_string())),
                    (s, Some(ArgType::StringType)) => Some(ArgVal::String(s.to_string())),
                    (s, Some(ArgType::BoolType)) => Some(ArgVal::Bool(s.to_string())),
                    (s, Some(ArgType::NumberType)) => Some(ArgVal::Number(s.to_string())),
                    (_, Some(ArgType::NullType)) => Some(ArgVal::Null),
                    (s, Some(ArgType::ArrayType)) => Some(ArgVal::Array(s.to_string())),
                    _ => None, // don't have a rep for fct values
                };

                sig_with_types.push(opt_ty);
                sig_with_values.push(opt_val);
            }

            res.push(Self {
                pkg: api_call.pkg,
                acc_path: match AccessPathModuleCentred::from_str(&api_call.acc_path) {
                    Ok(ap) => ap,
                    Err(_) =>
                    // if there's an unparsable access path, that's ok just don't add it to the list (i.e., skip it)
                    {
                        continue
                    }
                },
                sig_with_types,
                sig_with_values,
            });
        }
        Ok(res)
    }

    /// Turn a list of mined api calls into a map of lists indexed by the library
    /// the function originates from.
    pub fn lib_map_from_list(all_calls: Vec<Self>) -> LibMinedCallData {
        let mut ret_map = HashMap::new();
        for call in all_calls {
            ret_map
                .entry(call.get_pkg())
                .or_insert(Vec::new())
                .push(call);
        }
        ret_map
    }

    pub fn get_pkg(&self) -> String {
        self.pkg.clone()
    }

    pub fn get_acc_path(&self) -> AccessPathModuleCentred {
        self.acc_path.clone()
    }

    pub fn get_fct_name(&self) -> String {
        match &self.acc_path {
            AccessPathModuleCentred::FieldAccPath(_, FieldNameType::StringField(fct_name)) => {
                fct_name.clone()
            }
            _ => "".to_string(),
        }
    }
}
