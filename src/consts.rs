//! Configuration values for the test generator.
//! TODO these should be the default values, add user functionality
//! for specification via config file

// configuration for the test generator itself
/// Number of tests generated per function in the API discovery phase.
pub const DISCOVERY_PHASE_TESTING_BUDGET: i32 = 10;
/// Timeout (in seconds) after which an executing test is killed.
pub const TEST_TIMEOUT_SECONDS: u64 = 30;
/// If we specify a nested extension but there's no valid test that can be extended
/// in a nested way, what do we do? `false`: error, or `true`: just return a fresh test
pub const FRESH_TEST_IF_CANT_EXTEND: bool = true;

// restrictions on generated values
/// Allow generation of multiple callback arguments to the same function?
pub const ALLOW_MULTIPLE_CALLBACK_ARGS: bool = false;
/// Allow the generation of type `any` for random signatures.
pub const ALLOW_ANY_TYPE_ARGS: bool = true;
/// Upper bound (inclusive) for generated random numbers.
pub const MAX_GENERATED_NUM: f64 = 1000.0;
/// Upper bound (inclusive) for length of generated random arrays.
pub const MAX_GENERATED_ARRAY_LENGTH: usize = 10;
/// Upper bound (inclusive) for length (i.e., number of fields) of generated random objects.
pub const MAX_GENERATED_OBJ_LENGTH: usize = 5;
/// Upper bound (inclusive) for length of generated random strings.
pub const RANDOM_STRING_LENGTH: usize = 5;
/// Upper bound (inclusive) for the number of arguments generated for functions with
/// unspecified number of arguments.
pub const DEFAULT_MAX_ARG_LENGTH: usize = 5;

// choice percentages
// Chance of generating a new (i.e., not previously tested) signature.
pub const CHOOSE_NEW_SIG_PCT: f64 = 0.5;
// If we choose a function, now re-choosing is at its weight*<this>.
pub const RECHOOSE_LIB_FCT_WEIGHT_FACTOR: f64 = 0.8;
// If we choose a function signature, now re-choosing is at its weight*<this>.
pub const RECHOOSE_FCT_SIG_WEIGHT_FACTOR: f64 = 0.8;
// Chance of using a mined nesting example, if one is available.
pub const USE_MINED_NESTING_EXAMPLE: f64 = 0.5;
// Chance of using a mined API call signature example, if one is available.
pub const USE_MINED_API_CALL_SIG: f64 = 0.5;

/// Metadata for the file system setup required before tests are generated.
pub mod setup {
    /// Directories generated, to use in the tests.
    pub const TOY_FS_DIRS: [&str; 2] = ["a/b/test/directory", "a/b/test/dir"];
    /// Files generated, to use in the tests.
    pub const TOY_FS_FILES: [&str; 2] = ["a/b/test/directory/file.json", "a/b/file"];
    /// Name of the directory in which the generated tests are to be written.
    pub const TEST_DIR_PATH: &str = "test";
    /// Prefix for the file name of the generated tests.
    pub const TEST_FILE_PREFIX: &str = "test";
}
