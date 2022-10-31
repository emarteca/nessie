// representation of mined data
// TODO: in the improved version of the test generator, we're going to mine
// much more information -- the current struct representing a mined data point
// only represents nesting relationships, and is going to get totally overhauled

use crate::errors::*;
use crate::functions::{ArgType, FunctionArgument, FunctionSignature};
use crate::tests::FunctionCall;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::convert::TryFrom;
use std::path::PathBuf;

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
            }
        ]
    },
*/
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MinedNestingPairJSON {
    /// name of the module the outer call belongs to
    outer_pkg: String,
    /// name of the outer function call
    outer_fct: String,
    /// parameters to the outer function call
    outer_params: Vec<MinedParam>,
    /// name of the module the inner call belongs to
    inner_pkg: String,
    /// name of the inner function call
    inner_fct: String,
    /// parameters to the inner function call
    inner_params: Vec<MinedParam>,
}

pub type LibMinedData = HashMap<String, Vec<MinedNestingPairJSON>>;

impl MinedNestingPairJSON {
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

    pub fn get_outer_pkg(&self) -> String {
        // in the mined data, the package name is surrounded by "", so strip these
        self.outer_pkg.replace("\"", "")
    }

    pub fn get_inner_pkg(&self) -> String {
        // in the mined data, the package name is surrounded by "", so strip these
        self.inner_pkg.replace("\"", "")
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

/// TODO when we redo the data mining, the structure of the params should be better
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MinedParam {
    // the param is either an ident, a callback, or an object
    // if callback, the string is just "CALLBACK"
    // if object, the string is just "OBJECT
    // if ident, then it's the name of the parameter: either "outer_arg_" or "inner_arg_" followed by
    // the argument position.
    // The important part here is that it represents dataflow between the args of the outer to inner functions
    ident: Option<String>,
    callback: Option<String>,
    object: Option<String>,
}

impl MinedParam {
    pub fn is_valid(&self) -> bool {
        match (&self.ident, &self.callback, &self.object) {
            (Some(_), None, None) => true,
            (None, Some(_), None) => true,
            (None, None, Some(_)) => true,
            _ => false,
        }
    }

    pub fn is_callback(&self) -> bool {
        self.is_valid() && self.callback.is_some()
    }

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
            "{}",
            format!(
                "ident: {:?}, object: {:?}, callback: {:?}",
                self.ident, self.object, self.callback
            )
        )
    }
}

/// Representation of a generated inner function call, as a nested extension based
/// on some mined data.
/// It includes the name and signature of the inner function, and a list of the dataflow from
/// arguments to the outer call to this inner call.
#[derive(Debug, Clone)]
pub struct MinedDataNestedExtension {
    pub fct_name: String,
    pub sig: FunctionSignature,
    /// list of pairs of: position of argument in outer function call, passed to position in inner call
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

/// Given a list of mined data pairs and an outer function to extend,
/// return a random nested extension from the mined data (if one exists).
pub fn get_rel_mined_data_nested_extensions(
    outer_fct: Option<&FunctionCall>,
    pkg_name: &String,
    mined_data: &Vec<MinedNestingPairJSON>,
) -> Vec<MinedDataNestedExtension> {
    if !outer_fct.is_some() {
        return Vec::new();
    }
    let outer_fct = outer_fct.unwrap();
    // can't nest if the outer function has no callback argument
    if !outer_fct.sig.has_cb_arg() {
        return Vec::new();
    }
    let outer_arg_len = outer_fct.sig.get_arg_list().len();
    // println!("{:?}", outer_fct);
    let outer_fct_name = outer_fct.get_name();

    mined_data
        .iter()
        .filter_map(|mined_pair| {
            let inner_fct_sig = FunctionSignature::try_from(&mined_pair.inner_params);
            // note: right now we only support nestings from functions from the same package
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
                            let (_, outer_pos) = var_name.rsplit_once("_").unwrap();
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
