## `nessie` 

### System requirements
To use `nessie`, you'll need to install:
- [rustup](https://doc.rust-lang.org/cargo/getting-started/installation.html) 
- [nodejs](https://nodejs.org/en/download/)
- npm (installs with nodejs)
- [yarn](https://yarnpkg.com/) (you can install this with npm)

Note: `nessie` has only been tested on linux.
Some of the functionality might require some minor adapting to work for other OSs (e.g., the timeout for executing tests uses the linux `timeout` utility).

### Setup

First, clone this repo.
Then, `cd` to the root directory and build the test generator.

```
cargo build
```

### Usage 

```
cargo run -- --lib-name <name of package to generate tests for>
             --num-tests <number of tests to generate>
             [--lib-src-dir <path to directory for the source code of the package>] # optional
             [--testing-dir <path to directory where the generated tests should be written to] # optional
             [--module-import-code <path to file containing custom module import>] # optional
             [--mined-call-data ]
             [--mined-data <path to JSON file containing mined function nesting examples] # optional
             [--run-discover] # optional: flag to rerun the API discovery phase even if the discovery file exists
             [--skip-testgen] # optional: skip the test generation phase
             [--test-gen-mode <mode>] # optional: specify the mode to run the test generator in: OGNessie, TrackPrimitives, 
                                      # MergeDiscGen, ChainedMethods, or Head (the current head of the repo, and the default option)
```

The pipeline of `nessie` is as follows:
- If `lib-src-dir` is not included, install `lib-name` with `npm`
- Get the list of all API functions (by inspecting the properties of the module import)
  - output to a file `js_tools/<lib-name>_output.json` (if this file exists, future `nessie` runs will use it as input)
- Run the initial "simple" API discovery phase
  - systematically test signatures for API signatures and find positions and asynchronicity of callback arguments
  - output to a file `js_tools/<lib-name>_discovery.json` (if this file exists, future `nessie` runs will use it as input unless the `run-discover` option is specified)
- Run the test generation phase
  - iteratively construct `num-tests` tests for `lib-name`
  - for each test, collect the output as feedback and use this to build up a database of valid extension points of previous tests
  - for each test, also pass the test execution feedback to the advanced API discovery, learn new valid signatures/values and functions on return values, to add to the list of known signatures and test function receivers (if discovery mode is enabled with the current test generation mode)
  - for each test, inform the generation of signatures and argument values of the selected function to test have relevant mined data available
  - output these tests to `testing-dir` (the default value is a directory `test` in the `nessie` root directory)

*Note*: if using the source directory of a module with `lib-src-dir`, the `get_api_specs` script will try and install with the `yarn` package manager if `yarn.lock` exists, and tries to run `npm run build` if the `build` script is present. 
However, if the module has a custom build setup then you'll need to go in `get_api_specs` and change the build/setup code.


#### Example: generating 100 tests for `jsonfile` package

```
cargo run -- --lib-name jsonfile --num-tests 100
```
This generates 100 tests for `jsonfile` and outputs them to `test` directory.

#### Example: generating 100 tests for `jsonfile` from source, using `OGNessie` test generation mode 
First, clone the `jsonfile` source code and setup the project.
```
git clone https://github.com/jprichardson/node-jsonfile
cd node-jsonfile
npm install
cd ..
```
`jsonfile` doesn't have a build script and uses `npm` (i.e. not `yarn`) so at this point it's set up.

Then, run `nessie` using the local source of `jsonfile`, and specifying the test generation mode to be `OGNessie`
```
cargo run -- --lib-name jsonfile --num-tests 100 --lib-src-dir node-jsonfile --test-gen-mode OGNessie
```
This generates 100 tests for `jsonfile`, importing the local source directory of `jsonfile` as the module, and representing all paths as absolute paths in the file system.

##### Finding code coverage of `jsonfile` from the generated tests 
Generating tests with the local source of a module means you can use these as a test suite for the module itself.

To find the coverage these generated tests achieve on the local `jsonfile` source:

First, install `mocha` and `nyc` test library and coverage tools.
```
nyc mocha test/metatest.js
```
The resulting output displays the coverage of all source files in `node-jsonfile`.

### Contributing
This is an ongoing project! 
Feel free to reach out or make an issue or PR if you have improvement ideas.

### Academic work 
This is a Rust reimplementation of the test generator `nessie` presented in [my paper from ICSE 2022](https://conf.researchr.org/details/icse-2022/icse-2022-papers/69/Nessie-Automatically-Testing-JavaScript-APIs-with-Asynchronous-Callbacks).

To run the test generator only with the features that were present in the ICSE 2022 paper, specify the `test-gen-mode` argument to be `OGNessie`.
The changelog of the features included in the current test generator that were added onto the original test generator presented at ICSE 2022 are included [in these release notes](https://github.com/emarteca/nessie/tree/v1.0.0).

##### Evaluation metrics
This work has some associated academic publications; as part of these, we measured the effectiveness of the various modes of the test generator with respect to code coverage and the number of behavioural differences identified in a regression testing experiment. 
We've included the evaluation scripts in this repo for posterity, in case you want to redo any of experiments or reuse our evaluation technique.

Generate coverage:
```
# general case
./gen_cov_for_repo.sh <repo_link> <commit_hash> <number of tests to generate> <number of repetitions of this experiment> <test generation mode> [optional: <path to file of mined API call data>]

# specific example: 1000 tests, 10 reps, in OGNessie mode, for `node-glob` repo at a specified commit, with a file of mined data
./gen_cov_for_repo.sh https://github.com/isaacs/node-glob 8315c2d576f9f3092cdc2f2cc41a398bc656035a 1000 10 OGNessie mining/mined_all.json
```
The output will be a directory `TEST_REPO_<lib name>_all_tests`, which has a sub-directory for each repetition of the experiment that includes all of the tests.
The coverge output will be in a file `coverage.csv` in the root of this directory, with one line for each of the coverage values returned from this experiment.

Run regression testing experiment: 
```
# general case
./reg_test_for_repo_at_commits.sh <repo_link> <path to file with list of commits to test at> <number of tests to generate> <number of repetitions> <test generation mode>
```
The output will be the same directory structure and output as for the coverage experiment, except that the list of behavioural diffs will be included in a top-level (i.e., in the root of the test generated folder) `<lib name>_<num tests>_<commit>_<next commit>_seqDiffs` file.