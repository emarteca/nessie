// representation of mined data
// TODO: in the improved version of the test generator, we're going to mine
// much more information -- the current struct representing a mined data point
// only represents nesting relationships, and is going to get totally overhauled

use crate::errors::*;
use crate::functions::FunctionSignature;
use crate::tests::FunctionCall;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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

    pub fn lib_map_from_list(all_pairs: Vec<Self>) -> HashMap<String, Vec<Self>> {
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
}

pub struct MinedDataExtension {
    pub fct_name: String,
    pub sig: FunctionSignature,
    // pair of: position of argument in outer function call, passed to position in inner call
    pub outer_to_inner_dataflow: Option<(usize, usize)>,
}

pub fn choose_corresponding_mined_data(
    outer_fct: Option<&FunctionCall>,
    pkg_name: String,
    mined_data: Vec<MinedNestingPairJSON>,
) -> Option<MinedDataExtension> {
    None
}
