// given the name of an api/package,
// this script imports it, retrieves some information,
// and then dumps that to a JSON file

let fs = require("fs");

const DEFAULT_MAX_ARGS = 5;

// https://stackoverflow.com/a/54098693
function get_args () {
    const args = {};
    process.argv
        .slice(2, process.argv.length)
        .forEach( arg => {
        // args, all specified with --
        if (arg.slice(0,2) === '--') {
            const longArg = arg.split('=');
            const longArgFlag = longArg[0].slice(2,longArg[0].length);
            const longArgValue = longArg.length > 1 ? longArg[1] : true;
            args[longArgFlag] = longArgValue;
        }
    });
    return args;
}
const args = get_args();

// argument is the name of the lib to be processed
const libname = args["lib_name"];
let lib_require = "require(\"" + libname + "\");";
console.log(args);

// then we provide a source dir for the api code -- require this
if(args["lib_src_dir"] != "") {
	lib_require = "require(\"" + args["lib_src_dir"] + "\");";
}

// but if we specified a custom import for the module, then this takes precedence
if(args["import_code_file"] != "") {
	lib_require = fs.readFileSync(args["import_code_file"], 'utf-8');
}


// import the lib, then get info required
const lib = eval(lib_require);
let fn_names = Object.getOwnPropertyNames(lib).filter((p) => typeof lib[p] === 'function');


// for each function in the lib, get the number of arguments
let fn_info = {};
let default_acc_path = "(module " + libname + ")"; // module import
fn_names.forEach( name => {
	let cur_fn_info = {};
	cur_fn_info["num_args"] = lib[name] ? lib[name].length : 2; // if the function doesn't exist on the lib then give it 2 args 
	cur_fn_info["name"] = name;
	cur_fn_info["sigs"] = []; // start with no discovered signatures
	if(lib[name].toString().indexOf("...args") > -1) {
		cur_fn_info["num_args"] = DEFAULT_MAX_ARGS;
		cur_fn_info["used_default_args"] = true;
	}
	fn_info[name + ", " + default_acc_path] = cur_fn_info;
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