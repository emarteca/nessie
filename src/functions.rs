//! Representations of functions, callbacks, and all components
//! (arguments, values, signatures).

use crate::errors::*;
use crate::module_reps::AccessPathModuleCentred;

use serde::{Deserialize, Serialize};
use std::convert::TryFrom;

/// Representation of a single signature of a module function.
/// This includes the number and types of arguments, etc.
/// Note that functions may have multiple valid signatures.
#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq, Hash)]
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

impl From<&Vec<ArgType>> for FunctionSignature {
    fn from(arg_types: &Vec<ArgType>) -> Self {
        Self {
            arg_list: arg_types
                .iter()
                .map(|ty| FunctionArgument::new(*ty, None))
                .collect::<Vec<FunctionArgument>>(),
            call_test_result: None,
            is_spread_args: false,
        }
    }
}

impl FunctionSignature {
    /// Constructor.
    pub fn new(
        arg_list: &[FunctionArgument],
        call_test_result: Option<FunctionCallResult>,
    ) -> Self {
        Self {
            arg_list: arg_list.to_owned(),
            call_test_result,
            is_spread_args: false,
        }
    }

    /// Get the abstract (i.e., non-concrete, with no values) signature -- this is the list of
    /// argument types.
    pub fn get_abstract_sig(&self) -> Vec<ArgType> {
        self.arg_list
            .iter()
            .map(|arg| arg.get_type())
            .collect::<Vec<ArgType>>()
    }

    /// Get the positions of callback arguments for this function signature.
    pub fn get_callback_positions(&self) -> Vec<usize> {
        let mut posns = Vec::new();
        for (pos, arg) in self.arg_list.iter().enumerate() {
            if arg.is_callback() {
                posns.push(pos);
            }
        }
        posns
    }

    /// Get the list of `ArgVal` representations of all the arguments to the callback
    /// arguments in this signature.
    /// i.e., if the signature has arg values `(1, 2, (cb0, cb1) => {...})`
    /// where the 3rd argument is a callback, then this function will return the
    /// list of variables `[cb0, cb1]`.
    /// If there are multiple callback arguments in the signature, this function returns
    /// a merged list of all the callbacks' arguments.
    ///
    /// Note: this is designed for use in getting the list of all callback arguments in scope
    /// at a given extension point.
    pub fn get_all_cb_args_vals(&self, context_uniq_id: &String) -> Vec<ArgVal> {
        self.arg_list
            .iter()
            .filter_map(|arg| match arg.get_arg_val() {
                // get the list of all arguments for each callback arg
                Some(ArgVal::Callback(CallbackVal::RawCallback(cb))) => {
                    Some(cb.get_all_cb_args_vals(context_uniq_id))
                }
                _ => None,
            })
            .flatten()
            .collect::<Vec<ArgVal>>()
    }

    /// Does the signature have at least one callback argument?
    pub fn has_cb_arg(&self) -> bool {
        for arg in self.arg_list.iter() {
            if arg.is_callback() {
                return true;
            }
        }
        false
    }

    /// Getter for arg list.
    pub fn get_arg_list(&self) -> &Vec<FunctionArgument> {
        &self.arg_list
    }

    /// Mutable getter for the arg list.
    pub fn get_mut_args(&mut self) -> &mut Vec<FunctionArgument> {
        &mut self.arg_list
    }

    /// Getter for the result of calling the function with this signature.
    pub fn get_call_res(&self) -> Option<FunctionCallResult> {
        self.call_test_result
    }

    pub fn set_call_res(&mut self, res: FunctionCallResult) {
        self.call_test_result = Some(res);
    }
}

/// Default signature is empty, with the spread argument, and untested.
impl std::default::Default for FunctionSignature {
    fn default() -> Self {
        Self {
            arg_list: Vec::new(),
            call_test_result: None,
            is_spread_args: true,
        }
    }
}

/// Representation of a function argument: type and optional value.
#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq, Hash)]
pub struct FunctionArgument {
    /// Type of the argument.
    arg_type: ArgType,
    /// Optional value of the argument.
    arg_val: Option<ArgVal>,
}

impl FunctionArgument {
    /// Constructor.
    pub fn new(arg_type: ArgType, arg_val: Option<ArgVal>) -> Self {
        Self { arg_type, arg_val }
    }

    /// Is this argument a callback?
    pub fn is_callback(&self) -> bool {
        self.arg_type == ArgType::CallbackType
    }

    /// Getter for string representation of argument value.
    /// Also takes optional bits of code that will make up the instrumentation,
    /// if the string representation is being instrumented (this is not relevant
    /// for primitive argument values, but if a callback is being instrumented
    /// then the `extra_body_code` is extra instrumentation code to be included
    /// in the body of the function, and `context_uniq_id` is the unique ID of the
    /// function this argument is being passed to, which is information needed
    /// for the instrumentation).
    pub fn get_string_rep_arg_val(
        &self,
        extra_body_code: Option<String>,
        context_uniq_id: Option<String>,
        print_instrumented: bool,
    ) -> Option<String> {
        Some(self.arg_val.clone()?.get_string_rep(
            extra_body_code,
            context_uniq_id,
            print_instrumented,
        ))
    }

    /// Short string representation of the argument.
    /// We don't need any of the instrumentation info here, since function args print as `[function]`.
    pub fn get_string_rep_arg_val_short(&self) -> Option<String> {
        match self.arg_type {
            ArgType::CallbackType => Some("\"[function]\"".to_string()),
            _ => self.get_string_rep_arg_val(None, None, false),
        }
    }

    /// Setter for the value of this argument.
    /// Returns an error if the value `arg_val` is not compatible with the type of this arg.
    pub fn set_arg_val(&mut self, arg_val: ArgVal) -> Result<(), TestGenError> {
        if !(arg_val.get_type().can_be_repd_as(self.arg_type)) {
            return Err(TestGenError::ArgTypeValMismatch);
        }
        self.arg_val = Some(arg_val);
        Ok(())
    }

    /// Getter for the argument value.
    pub fn get_arg_val(&self) -> &Option<ArgVal> {
        &self.arg_val
    }

    /// Mutable getter for the argument value.
    pub fn get_arg_val_mut(&mut self) -> &Option<ArgVal> {
        &mut self.arg_val
    }

    /// Getter for the type of this argument.
    pub fn get_type(&self) -> ArgType {
        self.arg_type
    }

    /// Setter for the callback ID of this argument (this is a no-op if
    /// this argument is not a callback).
    /// Returns an error if the value of this argument is not set.
    pub fn set_cb_id(&mut self, cb_id: Option<String>) -> Result<(), TestGenError> {
        match self.arg_val.as_mut() {
            Some(arg_val) => {
                arg_val.set_cb_id(cb_id)?;
                Ok(())
            }
            _ => Err(TestGenError::ArgValNotSetYet),
        }
    }
}

/// List of types of arguments that are represented.
/// Note: this can be modified for an arbitrary amount of granularity;
/// so far we have mainly stuck to the default types available in JavaScript,
/// with the added distinction between generated callbacks and API library functions.
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArgType {
    /// Number.
    NumberType,
    /// String.
    StringType,
    /// Array.
    ArrayType,
    /// Non-callback, non-array, object.
    ObjectType,
    /// Generated callback (TODO maybe more granularity here).
    CallbackType,
    /// API library function -- distinct from callbacks, since we're not building them.
    LibFunctionType,
    /// The `any` dynamic type.
    AnyType,
}

impl ArgType {
    /// Return `true` if the receiver (`self`) can be represented
    /// by the other type `ot`.
    pub fn can_be_repd_as(&self, ot: Self) -> bool {
        *self == ot || ot == Self::AnyType
    }

    /// Is this a primitive type?
    pub fn is_not_callback(&self) -> bool {
        match *self {
            ArgType::CallbackType | ArgType::LibFunctionType => false,
            _ => true,
        }
    }
}

impl std::fmt::Display for ArgType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            ArgType::NumberType => write!(f, "num"),
            ArgType::StringType => write!(f, "string"),
            ArgType::ArrayType => write!(f, "array"),
            ArgType::ObjectType => write!(f, "object"),
            ArgType::CallbackType => write!(f, "callback-function"),
            ArgType::LibFunctionType => write!(f, "lib-function"),
            ArgType::AnyType => write!(f, "any"),
        }
    }
}

/// Kinds of argument values.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum ArgVal {
    /// Number.
    Number(String),
    /// String.
    String(String),
    /// Array.
    Array(String),
    /// Non-callback, non-array, object.
    Object(String),
    /// Generated callback.
    Callback(CallbackVal),
    /// API library function.
    LibFunction(String),
    /// Variable (so far, this means scope-available previous function return or callback argument).
    Variable(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct ArgValAPTracked {
    pub(crate) val: ArgVal,
    pub(crate) acc_path: Option<AccessPathModuleCentred>,
}

impl ArgVal {
    /// Get the string representation of this argument value.
    /// Instrumentation code is passed in and used to instrument callback values.
    pub fn get_string_rep(
        &self,
        extra_body_code: Option<String>,
        context_uniq_id: Option<String>,
        print_instrumented: bool,
    ) -> String {
        match self {
            Self::Number(s)
            | Self::String(s)
            | Self::Array(s)
            | Self::Object(s)
            | Self::LibFunction(s)
            | Self::Variable(s) => s.clone(),
            Self::Callback(cbv) => {
                cbv.get_string_rep(extra_body_code, context_uniq_id, print_instrumented)
            }
        }
    }

    /// Get the type of this argument.
    /// `ArgVal` closely follows the `ArgType` enum, with the main distinction
    /// being the lack of value corresponding to the `any` type, and the addition
    /// of the `Variable` kind.
    /// Here, we consider `Variable` values to have the `any` type; this is used
    /// when passing previous return/callback arg values to later function calls.
    pub fn get_type(&self) -> ArgType {
        match self {
            Self::Number(_) => ArgType::NumberType,
            Self::String(_) => ArgType::StringType,
            Self::Array(_) => ArgType::ArrayType,
            Self::Object(_) => ArgType::ObjectType,
            Self::Callback(_) => ArgType::CallbackType,
            Self::LibFunction(_) => ArgType::LibFunctionType,
            Self::Variable(_) => ArgType::AnyType,
        }
    }

    /// Setter for the callback ID.
    /// Returns an error if the value is not a callback.
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

/// Kinds of callback value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum CallbackVal {
    /// Variable: if the callback is stored as a named function earlier, and represented by name.
    Var(String),
    /// Anonymous callback, represented as the raw signature/function-body.
    RawCallback(Callback),
}

impl CallbackVal {
    /// Get the string representation of this callback value.
    /// Instrumentation code is passed in and used to instrument raw callback values.
    pub fn get_string_rep(
        &self,
        extra_body_code: Option<String>,
        context_uniq_id: Option<String>,
        print_instrumented: bool,
    ) -> String {
        match self {
            Self::Var(vs) => vs.clone(),
            Self::RawCallback(cb) => {
                cb.get_string_rep(extra_body_code, context_uniq_id, print_instrumented)
            }
        }
    }
}

/// Representation of a callback function.
/// This is used to represent the generated callback arguments to library function calls.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct Callback {
    /// Signature of the callback function.
    pub(crate) sig: FunctionSignature,
    /// Unique ID, used in instrumentation.
    cb_id: Option<String>,
    /// The argument position that this callback occupies, in the function call
    /// for which this is an argument.
    pub(crate) cb_arg_pos: Option<usize>,
}

impl Callback {
    /// Constructor.
    pub fn new(sig: FunctionSignature) -> Self {
        Self {
            sig,
            cb_id: None,
            cb_arg_pos: None,
        }
    }

    /// Setter for the unique ID of this callback.
    pub fn set_cb_id(&mut self, cb_id: Option<String>) {
        self.cb_id = cb_id;
    }

    /// Setter for the argument position that this callback occupies.
    pub fn set_cb_arg_pos(&mut self, cb_arg_pos: Option<usize>) {
        self.cb_arg_pos = cb_arg_pos;
    }

    /// Getter for the list of all the arguments to this callback.
    /// This is the list of parameter names in this callback's signature.
    pub fn get_all_cb_args_vals(&self, context_uniq_id: &str) -> Vec<ArgVal> {
        let cb_arg_name_base = self.get_cb_arg_name_base(&Some(context_uniq_id.to_owned()));
        self.sig
            .get_arg_list()
            .iter()
            .enumerate()
            .map(|(pos, _)| ArgVal::Variable(cb_arg_name_base.clone() + &pos.to_string()))
            .collect::<Vec<ArgVal>>()
    }
}

/// Default callback is just empty, with the default signature (empty, with the spread argument).
impl std::default::Default for Callback {
    fn default() -> Self {
        Self {
            sig: FunctionSignature::default(),
            cb_id: None,
            cb_arg_pos: None,
        }
    }
}
