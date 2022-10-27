
let orig_log = console.log;
let output_log = [];
console.log = function(e) {
	output_log.push(e);
}
let jsonfile = require("jsonfile");
try { 

	console.log({"before_cb_ret_val_jsonfile_arg0": "js_tools/toy_fs_dir/a/b/test/directory"});
	console.log({"before_cb_ret_val_jsonfile_arg1": "js_tools/toy_fs_dir/a/b/test/dir"});
	console.log({"before_cb_ret_val_jsonfile_arg2": "[function]"});
	console.log({"before_cb_ret_val_jsonfile_arg3": "js_tools/toy_fs_dir/a/b/test/directory"});
	console.log({"before_cb_ret_val_jsonfile_arg4": "js_tools/toy_fs_dir/a/b/test/directory/file.json"});
	let ret_val_jsonfile = jsonfile.writeFile("js_tools/toy_fs_dir/a/b/test/directory", "js_tools/toy_fs_dir/a/b/test/dir", (	try { 

	console.log({"before_cb_ret_val_jsonfile_arg0": "js_tools/toy_fs_dir/a/b/test/directory/file.json"});
	console.log({"before_cb_ret_val_jsonfile_arg1": "js_tools/toy_fs_dir/a/b/file"});
	console.log({"before_cb_ret_val_jsonfile_arg2": "[function]"});
	console.log({"before_cb_ret_val_jsonfile_arg3": "js_tools/toy_fs_dir/a/b/test/dir"});
	console.log({"before_cb_ret_val_jsonfile_arg4": "js_tools/toy_fs_dir/a/b/test/directory"});
	let ret_val_jsonfile = jsonfile.writeFile("js_tools/toy_fs_dir/a/b/test/directory/file.json", "js_tools/toy_fs_dir/a/b/file", (
cb_arg_0, cb_arg_1, cb_arg_2
) => {
	console.log({"in_cb_arg_0": cb_arg_0});
	console.log({"in_cb_arg_1": cb_arg_1});
	console.log({"in_cb_arg_2": cb_arg_2});
	console.log({"callback_exec_2": 2});
}, "js_tools/toy_fs_dir/a/b/test/dir", "js_tools/toy_fs_dir/a/b/test/directory");
	console.log({"after_cb_ret_val_jsonfile_arg0": "js_tools/toy_fs_dir/a/b/test/directory/file.json"});
	console.log({"after_cb_ret_val_jsonfile_arg1": "js_tools/toy_fs_dir/a/b/file"});
	console.log({"after_cb_ret_val_jsonfile_arg2": "[function]"});
	console.log({"after_cb_ret_val_jsonfile_arg3": "js_tools/toy_fs_dir/a/b/test/dir"});
	console.log({"after_cb_ret_val_jsonfile_arg4": "js_tools/toy_fs_dir/a/b/test/directory"});
	console.log({"ret_val_jsonfile": typeof ret_val_jsonfile == "function"? "[function]" : ret_val_jsonfile.toString()});
	console.log({"ret_val_type": typeof ret_val_jsonfile});
	Promise.resolve(ret_val_jsonfile).catch(e => { console.log({"error_2": true}); });
} catch(e) {
	console.log({"error_2": true});
}
console.log({"done_2": true});)
, "js_tools/toy_fs_dir/a/b/test/directory", "js_tools/toy_fs_dir/a/b/test/directory/file.json");
	console.log({"after_cb_ret_val_jsonfile_arg0": "js_tools/toy_fs_dir/a/b/test/directory"});
	console.log({"after_cb_ret_val_jsonfile_arg1": "js_tools/toy_fs_dir/a/b/test/dir"});
	console.log({"after_cb_ret_val_jsonfile_arg2": "[function]"});
	console.log({"after_cb_ret_val_jsonfile_arg3": "js_tools/toy_fs_dir/a/b/test/directory"});
	console.log({"after_cb_ret_val_jsonfile_arg4": "js_tools/toy_fs_dir/a/b/test/directory/file.json"});
	console.log({"ret_val_jsonfile": typeof ret_val_jsonfile == "function"? "[function]" : ret_val_jsonfile.toString()});
	console.log({"ret_val_type": typeof ret_val_jsonfile});
	Promise.resolve(ret_val_jsonfile).catch(e => { console.log({"error_1": true}); });
} catch(e) {
	console.log({"error_1": true});
}
console.log({"done_1": true});

process.on("exit", function f() {
	orig_log(JSON.stringify(output_log));
})