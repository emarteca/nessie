use crate::errors::*;
/// functions, callbacks, and all components (arguments, values, signatures)
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;

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

    pub fn get_all_cb_args_vals(&self, context_uniq_id: &String) -> Vec<ArgVal> {
        self.arg_list
            .iter()
            .filter_map(|arg| match arg.get_arg_val() {
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
    pub fn get_string_rep_arg_val(
        &self,
        extra_body_code: Option<String>,
        context_uniq_id: Option<String>,
        print_instrumented: bool,
    ) -> Option<String> {
        Some(
            self.arg_val
                .clone()?
                .get_string_rep(extra_body_code, context_uniq_id, print_instrumented)
                .clone(),
        )
    }

    // don't need any of the function ID stuff here, since functions just print as "[function]"
    pub fn get_string_rep_arg_val__short(&self) -> Option<String> {
        match self.arg_type {
            ArgType::CallbackType => Some("\"[function]\"".to_string()),
            _ => self.get_string_rep_arg_val(None, None, false).clone(),
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
    /// library function -- distinct from callbacks, since we're not building them
    LibFunctionType,
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
            ArgType::CallbackType => write!(f, "callback-function"),
            ArgType::LibFunctionType => write!(f, "lib-function"),
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
    LibFunction(String),
    Variable(String),
}

impl ArgVal {
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Callback {
    pub(crate) sig: FunctionSignature,
    // unique ID, used when printing to determine what the CB is
    cb_id: Option<String>,
    // the argument position that this callback is in
    pub(crate) cb_arg_pos: Option<usize>,
}

impl Callback {
    pub fn new(sig: FunctionSignature) -> Self {
        Self {
            sig,
            cb_id: None,
            cb_arg_pos: None,
        }
    }

    pub fn set_cb_id(&mut self, cb_id: Option<String>) {
        self.cb_id = cb_id;
    }

    pub fn set_cb_arg_pos(&mut self, cb_arg_pos: Option<usize>) {
        self.cb_arg_pos = cb_arg_pos;
    }

    pub fn get_all_cb_args_vals(&self, context_uniq_id: &String) -> Vec<ArgVal> {
        let cb_arg_name_base = self.get_cb_arg_name_base(&Some(context_uniq_id.clone()));
        self.sig
            .get_arg_list()
            .iter()
            .enumerate()
            .map(|(pos, _)| ArgVal::Variable(cb_arg_name_base.clone() + &pos.to_string()))
            .collect::<Vec<ArgVal>>()
    }
}

impl std::default::Default for Callback {
    fn default() -> Self {
        Self {
            sig: FunctionSignature::default(),
            cb_id: None,
            cb_arg_pos: None,
        }
    }
}
