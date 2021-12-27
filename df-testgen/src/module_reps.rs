/// the data structures representing a module

use std::collections::HashMap;
use std::{fs, io, path::PathBuf};

use serde::{Deserialize, Serialize};

/// Module class:
/// - represents the library 
/// - composed of a list of functions
/// - each function is composed of a list of signatures
pub struct NpmModule {
	/// name of the npm module 
	name: String,
	/// map of functions making up the module
	/// indexed by the name of the function
	fct_list: HashMap<String, ModuleFunction>,
}

impl NpmModule {
	pub fn from_api_spec(path: PathBuf, mod_name: String) -> Result<Self, DFError> {
		let file = fs::File::open(path.clone());
		if let Err(_) = file {
			return Err(DFError::SpecFileError);
		}
		let file = file.unwrap();
	    let reader = io::BufReader::new(file);

	    print!("{:?}", reader);

	    Ok(Self{
	    	name: mod_name,
	    	fct_list: HashMap::new(),
	    })
	}
}

/// representation of a function in a given module
/// each function has a list of valid signatures
pub struct ModuleFunction {
	/// name of the function
	name: String,
	/// list of valid signatures
	sigs: Vec<FunctionSignature>,
} 

/// representation of a single signature of a module function
/// this includes the number and types of arguments, etc
/// note that functions may have multiple valid signatures
pub struct FunctionSignature {
	/// number of arguments
	num_args: i32,
	/// is it async? true/false
	is_async: bool,
	/// list of arguments: their type, and value if tested
	arg_list: Vec<FunctionArgument>, 
}

impl FunctionSignature {
	/// get the positions of callback arguments for this function
	pub fn get_callback_positions(&self) -> Vec<usize> {
		let mut posns = Vec::new();
		for (pos, arg) in self.arg_list.iter().enumerate() {
			if (arg.is_callback) {
				posns.push(pos);
			}
		}
		posns
	}
}

/// representation of a function argument
pub struct FunctionArgument {
	/// type of the argumnet
	arg_type: ArgType,
	/// is this argument a callback? true/false
	is_callback: bool,
	// if tested, list of values tested with
	// TODO figure out how to represent these values
}	

/// list of types being tracked, for arguments
/// this can be modified for an arbitrary amount of granularity
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
	ObjectType
}

/// errors in the DF testgen pipeline
pub enum DFError {
	/// error reading some sort of spec file from a previous stage of the pipeline 
	SpecFileError,
}