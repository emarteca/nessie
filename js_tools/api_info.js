// given the name of an api/package,
// this script imports it, retrieves some information,
// and then dumps that to a JSON file

let fs = require("fs");

const DEFAULT_MAX_ARGS = 5;




// argument is the name of the lib to be processed
const libname = process.argv[2];
let lib_require = libname;
// then we provide a source dir for the api code -- require this
if(process.argv.length == 4) {
	lib_require = process.argv[3];
}

// import the lib, then get info required
const lib = require(lib_require);
let fn_names = Object.getOwnPropertyNames(lib).filter((p) => typeof lib[p] === 'function');

// for each function in the lib, get the number of arguments
let fn_info = {};
fn_names.forEach( name => {
	let cur_fn_info = {};
	cur_fn_info["num_args"] = lib[name] ? lib[name].length : 2; // if the function doesn't exist on the lib then give it 2 args 
	cur_fn_info["name"] = name;
	if(lib[name].toString().indexOf("...args") > -1) {
		cur_fn_info["num_args"] = DEFAULT_MAX_ARGS;
		cur_fn_info["used_default_args"] = true;
	}
	fn_info[name] = cur_fn_info;
});


// set up output json object
let output_obj = {
	lib: libname,
	fns: fn_info,
}

// and print it to a file
fs.writeFile(libname + "_output.json", JSON.stringify(output_obj, null, 2), (err) => {
	if (err) throw err;
	console.log("done processing " + libname);
});