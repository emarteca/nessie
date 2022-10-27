let fs = require("fs");

let orig_log = console.log;
let output_log = [];
console.log = function(e) {
	output_log.push(e);
}

let cb = function() { console.log({"callback_exec": true}); }
let base = fs;
try {
	let ret_val = fs.readFile("api_info.js", cb);
	console.log({"api_ret_val": ret_val});
} catch(e) {
	console.log({"error": true});
}
console.log({"done_test": true});

process.on("exit", function f() {
	orig_log(output_log);
})


let callback = () => {console.log("Callback executed");}
try {
  // try calling the specified API function
  api(..., callback, ...);
  console.log("API call executed");
} catch(e) {
  console.log("Error in API call");
} 
console.log("Test executed");


let fs_extra = require("fs-extra");

// read the contents of file.json and output it to output.json
fs_extra.readJson ("file.json" , function callback ( err , obj ) {
	fs_extra.outputJson ("output.json" , obj );
	console.log("Done reading the file!");
});

console.log("Done the program!");


let fs_extra = require("fs-extra");

// read the contents of file.json 
fs_extra.readJson ("file.json" , function callback ( err , obj ) {
	console.log("Done reading the file!");
});

console.log("Done the program!");

// >> Done the program!
// >> Done reading the file!



// Omitted for clarity:
//  * Try-catch around each call to an API function.
//  * Print statements to log arguments and return values.
//  * Print statements to log control flow.

let fs_extra = require("fs-extra");

var arg590 = "a/b/test";
var arg593 = null;

let r_126_0 = fs_extra.ensureFile(arg590, cb(a, b, c, d, e) => {
  let r_126_0_0 = fs_extra.readJson(arg590);
  return false;
});

let r_126_1 = fs_extra.stat(arg593);


// extended
let fs_extra = require("fs-extra");

var arg590 = "a/b/test";
var arg593 = null;
var argNEWARG1 = "a/b/new_file.json";

let r_126_0 = fs_extra.ensureFile(arg590, cb(a, b, c, d, e) => {
  let r_126_0_0 = fs_extra.readJson(arg590);
  let r_126_0_1 = fs_extra.outputJson(argNEWARG1, r_126_0_0);
  return false;
});

let r_126_1 = fs_extra.stat(arg593);





// original test with prints
let fs_extra = require("fs-extra");

var arg590 = "a/b/test";
var arg593 = null;

let r_126_0 = null;
let r_126_0_0 = null;
let r_126_1 = null;

try {
	r_126_0 = fs_extra.ensureFile(arg590, cb(a, b, c, d, e) => {
	  console.log("Value of callback argument a: " + (typeof a == "function" ? "function" : a))
	  console.log("Value of callback argument b: " + (typeof b == "function" ? "function" : b))
	  console.log("Value of callback argument c: " + (typeof c == "function" ? "function" : c))
	  console.log("Value of callback argument d: " + (typeof d == "function" ? "function" : d))
	  console.log("Value of callback argument e: " + (typeof e == "function" ? "function" : e))
	  console.log("Callback executing in method: ensureFile");	
	  try {
	  	r_126_0_0 = fs_extra.readJson(arg590);
	  	console.log("Finished calling: fs_extra.readJson")
	  	console.log("Value of r_126_0_0: " + r_126_0_0);
	  } catch(e) {
	  	console.log("Error calling: fs_extra.readJson");
	  }
	  console.log("Callback returning false: " + (typeof false == "function" ? "function" : false));
	  return false;
	});
	console.log("Finished calling: fs_extra.ensureFile");
	console.log("Value of r_126_0: " + r_126_0);
} catch(e) {
	console.log("Error calling: fs_extra.ensureFile");
}

try {
	r_126_1 = fs_extra.stat(arg593);
	console.log("Finished calling: fs_extra.stat");
	console.log("Value of r_126_1: " + r_126_1);
} catch(e) {
	console.log("Error calling: fs_extra: stat");
}


// nesting example
function callback(err, files) {
	files.forEach( file => fs.readFile(file, ...));
}

fs.readdir(".", callback(err, files) {
	files.forEach( file => fs.readFile(file, (cont) => console.log(cont)));
});


