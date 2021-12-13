// given the name of an api/package,
// this script imports it, retrieves some information,
// and then dumps that to a JSON file

let fs = require("fs");

const libname = process.argv[2];

const lib = require(libname);
console.log(lib);

let fnNames = Object.getOwnPropertyNames(lib).filter((p) => typeof lib[p] === 'function');




fs.writeFile(libname + "_output.json", JSON.stringify(fnNames, null, 2), (err) => {
	if (err) throw err;
	console.log("done processing " + lib);
});