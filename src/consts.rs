pub const DISCOVERY_PHASE_TESTING_BUDGET: i32 = 3;
pub const ALLOW_MULTIPLE_CALLBACK_ARGS: bool = false;
pub const ALLOW_ANY_TYPE_ARGS: bool = true;
pub const TEST_TIMEOUT_SECONDS: u64 = 30;

pub const MAX_GENERATED_NUM: f64 = 1000.0;
pub const MAX_GENERATED_ARRAY_LENGTH: usize = 10;
pub const MAX_GENERATED_OBJ_LENGTH: usize = 5;
pub const RANDOM_STRING_LENGTH: usize = 5;
pub const DEFAULT_MAX_ARG_LENGTH: usize = 5;

// choice percentages
pub const CHOOSE_NEW_SIG_PCT: f64 = 0.5; // chance of new signature
pub const RECHOOSE_LIB_FCT_WEIGHT_FACTOR: f64 = 0.8; // if we choose a function, now re-choosing is at its weight*<this>
pub const USE_MINED_NESTING_EXAMPLE: f64 = 0.5; // chance of using a mined nesting example, if one is available

/// metadata for the setup required before tests are generated
pub mod setup {
    pub const TOY_FS_DIRS: [&str; 2] = ["a/b/test/directory", "a/b/test/dir"];
    pub const TOY_FS_FILES: [&str; 2] = ["a/b/test/directory/file.json", "a/b/file"];
    pub const TEST_DIR_PATH: &str = "js_tools";
    pub const TEST_FILE_PREFIX: &str = "test";
}
