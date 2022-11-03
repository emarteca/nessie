## `nessie` 

### System requirements
To use `nessie`, you'll need to install:
- [rustup](https://doc.rust-lang.org/cargo/getting-started/installation.html) 
- [nodejs](https://nodejs.org/en/download/)
- npm (installs with nodejs)

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
             [--mined-data <path to JSON file containing mined function nesting examples] # optional
             [--run-discover] # optional: flag to rerun the API discovery phase even if the discovery file exists
             [--skip-testgen] # optional: skip the test generation phase
```

The pipeline of `nessie` is as follows:
- If `lib-src-dir` is not included, install `lib-name` with `npm`
- Get the list of all API functions (by inspecting the properties of the module import)
  - output to a file `js_tools/<lib-name>_output.json` (if this file exists, future `nessie` runs will use it as input)
- Run the API discovery phase
  - systematically test signatures for API signatures and find positions and asynchronicity of callback arguments
  - output to a file `js_tools/<lib-name>_discovery.json` (if this file exists, future `nessie` runs will use it as input unless the `run-discover` option is specified)
- Run the test generation phase
  - iteratively construct `num-tests` tests for `lib-name`
  - for each test, collect the output as feedback and use this to build up a database of valid extension points of previous tests
  - output these tests to `testing-dir` (the default value is a directory `test` in the `nessie` root directory)


#### Example: generating 100 tests for `jsonfile` package

```
cargo run -- --lib-name jsonfile --num-tests 100
```
This generates 100 tests for `jsonfile` and outputs them to `test` directory.

#### Example: generating 100 tests for `jsonfile` from source
First, clone the `jsonfile` source code and setup the project.
```
git clone https://github.com/jprichardson/node-jsonfile
cd node-jsonfile
npm install
cd ..
```
Then, run `nessie` using the local source of `jsonfile`.
```
cargo run -- --lib-name jsonfile --num-tests 100 --lib-src-dir node-jsonfile
```
This generates 100 tests for `jsonfile`, importing the local source directory of `jsonfile` as the module, and representing all paths as absolute paths in the file system.

##### Finding code coverage of `jsonfile` from the generated tests 
Generating tests with the local source of a module means you can use these as a test suite for the module itself.

To find the coverage these generated tests achieve on the local `jsonfile` source:

First, install `mocha` and `nyc` test library and coverage tools.
```
cd node-jsonfile
rm test/* # get rid of the existing test suite, or just move it
cp ../test/*.js test # copy over our generated tests
nyc mocha test/metatest.js
```
The resulting output displays the coverage of all source files in `node-jsonfile`.

### TODOs / contributing
This is an ongoing project! 
Feel free to reach out or make an issue or PR if you have improvement ideas.

Some improvements we're already planning / working on:
* adding support for chained function call generation (e.g., promise chains)
* adding support for modules that are represented differently than just the base import of the package
* merging the API discovery and test generation phases
* incorporating more type information into the feedback for signatures (i.e., more than just callback/not-callback and asynchronicity of calls)
* more advanced static analysis for mined data (right now only nesting pairs are supported)

### Academic work 
This is a Rust reimplementation of [my paper from ICSE 2022](https://conf.researchr.org/details/icse-2022/icse-2022-papers/69/Nessie-Automatically-Testing-JavaScript-APIs-with-Asynchronous-Callbacks).
Once we are done the proposed improvements and release a new version of `nessie`, we'll release a corresponding changelog from the original work presented at ICSE.