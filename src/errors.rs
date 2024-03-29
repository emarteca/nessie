//! Custom errors and results for the test generator.

use crate::tests::ExtensionType;
use serde::{Deserialize, Serialize};

/// Errors in the Data- and Feedback- driven test generation pipeline
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum DFError {
    /// error reading some sort of spec file from a previous stage of the pipeline
    SpecFileError,
    /// error reading the mined data file
    MinedDataFileError,
    /// error in the mined data, with an error message
    InvalidMinedData(String),
    /// error printing test file (with file path)
    WritingTestError(String),
    /// error deleting test file (with file path)
    DeletingTestError(String),
    /// error running test (could be a timeout)
    TestRunningError,
    /// error parsing test output
    TestOutputParseError,
    /// invalid test extension option
    InvalidTestExtensionOption,
    /// error during test generation
    TestGenError(TestGenError),
}

/// Errors in the test generation.
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

/// Representation of the different test outcomes we're tracking.
/// TODO might track other outcomes in the extended test generator.
/// In this case, the only test is only about the callback arguments (whether or not
/// they were called, and in what order).
#[derive(Debug, PartialEq, Eq, Clone, Copy, Deserialize, Serialize, Hash)]
pub enum SingleCallCallbackTestResult {
    /// callback is called and executed synchronously, and no error
    CallbackCalledSync,
    /// callback is called and executed asynchronously, and no error
    CallbackCalledAsync,
    /// callback is not called, and no error
    NoCallbackCalled,
}

/// Possible results of one function execution.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Deserialize, Serialize, Hash)]
pub enum FunctionCallResult {
    /// result WRT whether a callback argument was executed when this function is called
    SingleCallback(SingleCallCallbackTestResult),
    /// there is an error in the execution of the function
    ExecutionError,
    // TODO MultiCallback
}

impl FunctionCallResult {
    /// Test if the function result means the function that was executed can be
    /// used as an extension point of type `ExtensionType`.
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
