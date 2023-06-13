import argparse
import sys
import subprocess
import re
import json
import numpy as np

def run_command( command, timeout=None):
	try:
		process = subprocess.run( command.split(), stdout=subprocess.PIPE, stdin=subprocess.PIPE, stderr=subprocess.PIPE, timeout=timeout)
	except Exception as e:
		error = "\nError running: " + command
		print(error)
		print(e)
		return( error.encode('utf-8'), error.encode('utf-8'), 1) # non-zero return code
	return( process.stderr, process.stdout, process.returncode)

def diagnose_diff( diff_string):
    # [commit_newv, commit_oldv] = diff_string.split("\n---\n")
	split_out = diff_string.split("\n---\n")
	commit_newv = "" if len(split_out) == 1 and not split_out[0].startswith("< ") else split_out[0]
	commit_oldv = "" if len(split_out) == 1 and not split_out[0].startswith("> ") else split_out[0] if len(split_out) == 1 else split_out[1]
    # get rid of the json noise
	commit_newv.replace("> [", "> ")
	commit_newv.replace("> {", "> ")
	commit_newv.replace("< [", "< ")
	commit_newv.replace("< {", "< ")
	commit_oldv.replace("> [", "> ")
	commit_oldv.replace("> {", "> ")
	commit_oldv.replace("< [", "< ")
	commit_oldv.replace("< {", "< ")

	if commit_newv.startswith("< done_") and commit_oldv.startswith("> error_"):
		return( "Call_fails_oldv" + ": " + commit_newv.split(".")[1].split("\n")[0]) # first "." is base.methodName
	elif commit_newv.startswith("< done_") and commit_oldv.startswith("> error_"):
		return( "Call_fails_newv" + ": " + commit_oldv.split(".")[1].split("\n")[0])
	elif commit_newv.startswith("< done_") and commit_oldv.startswith("> done_"):
		return( "Diff_internal_name" + ": " + commit_oldv.split(".")[1].split("\n")[0])
	elif commit_newv.startswith("< ret_val_") and commit_oldv.startswith("> ret_val_"):
		return( "Diff_return_value")
	# ordering is important here: if the difference is not a return value (i.e. check after return)
	# and the difference is not caught by one of the more general argument clauses below
	# this is to diagnose an API function as being undefined (i.e. doesnt exist) in one case
	elif commit_newv.startswith("< after_") and commit_oldv.startswith("> after_") and commit_newv.split(": ")[1].startswith("undefined"):
		return( "API_func_no_longer_exists")
	elif commit_newv.startswith("< before_") and commit_oldv.startswith("> before_"):
		start_of_arg = commit_newv.split(": ")[1]
		if start_of_arg.startswith("class ") or start_of_arg.startswith("function ") or start_of_arg.startswith("async ") or start_of_arg.startswith("("):
			return("Function_arg_impl_diff")
		return( "Diff_API_argument_value")
	elif commit_newv.startswith("< in_") and commit_oldv.startswith("> in_"):
		start_of_arg = commit_newv.split(": ")[1]
		if start_of_arg.startswith("class ") or start_of_arg.startswith("function ") or start_of_arg.startswith("async ") or start_of_arg.startswith("("):
			return("Function_arg_impl_diff")
		return( "Diff_callback_argument_value")
	elif commit_newv.startswith("< callback_exec_"):
		return( "Callback_called_newv_notcalled_oldv" + ": " + commit_newv.split("< callback_exec_")[1].split("\n")[0]) # name of method
	elif commit_newv.startswith("< async_error_in_test"):
		return( "Internal_async_error_newv")
	elif commit_oldv.startswith("> callback_exec_ "):
		return( "Callback_notcalled_newv_called_oldv" + ": " + commit_oldv.split("> callback_exec_")[1].split("\n")[0]) # name of method
	elif commit_oldv.startswith("> async_error_in_test"):
		return( "Internal_async_error_oldv")	
	elif commit_newv.startswith("<     ✓") and commit_oldv.startswith(">     ✓"): # timing artifact: meaningless diff
		return( None)
	elif commit_newv.startswith("< \n<   test") or commit_oldv.startswith("> \n>   test"): # whitespace artifact on test completion: meaningless diff
		return( None)
	elif commit_newv.find("Cannot find module '.") != -1: # this indicates a local module dependency that is not available in the newer commit 
		return( "Local_file_renamed_or_removed")		  # happens when a file is renamed or deleted. Will not happen with commit_oldv since the tests are gen'd for this commit
	elif commit_newv.find("Cannot find module '") != -1: # this indicates a NON-local module dependency that is not available in the newer commit 
		return( "Nonlocal_dependency_removed")   		 # note: this needs to be *after* the previous check, since it would also catch the missing local modules
	elif commit_newv.find("ReferenceError: primordials is not defined") != -1: # common error indicating a mismatch between grub and nodejs versions
		return( "Grub_node_version_mismatch")								   # see: https://stackoverflow.com/questions/55921442/how-to-fix-referenceerror-primordials-is-not-defined-in-node
	elif commit_newv.find("ReferenceError: ") != -1: # environment reference not included (primordials is one example)
		return( "Env_ref_not_included")	
	elif commit_newv.find("SyntaxError: ") != -1: # syntax error in new tests, due to language upgrade (probably CJS --> ESM)
		return( "Syntax_err")	
	else:
		return("CATCHALL_UNDIAGNOSED")

def remove_noise_diffs( diff_list):
	noise_diffs = ["Local_file_renamed_or_removed", "Nonlocal_dependency_removed", "Grub_node_version_mismatch", "Env_ref_not_included", "Syntax_err"]
	if np.array([ noise_diffs.count(d) > 0 for d in diff_list]).any():
		return( [ d for d in diff_list if d != "CATCHALL_UNDIAGNOSED" and not d.split(":")[0].endswith("oldv")]) # if the new commit test doesnt run, other diffs are meaningless
	return( diff_list)

# no need to return since the list is passed by ref and modified in method
def prune_until_equal( to_prune, elt1, elt2):
	num_to_rem = min( to_prune.count(elt1), to_prune.count(elt2))
	for i in range(num_to_rem):
		to_prune.remove(elt1)
		to_prune.remove(elt2)	

def diagnose_all_diffs( diff_output):
	diffs = [l for l in re.compile("(^|\n)[0-9]+.*\n").split( diff_output) if len(l) > 0 and not l.isspace()]
	diagnosed = [ diagnose_diff(d) for d in diffs]
	# if the only diff is an equal number of Internal_async_error_newv and Internal_async_error_oldv
	# then this is indicative of a diff only due to the ordering of async executions 
	# so, we remove matching internal async error counts	
	prune_until_equal(diagnosed, "Internal_async_error_oldv", "Internal_async_error_newv")
	#spec_calls = [d.split(": ")[1] for d in diagnosed if d is not None and d.startswith("Callback_called_newv_notcalled_oldv")]
	#for call in spec_calls:
	#	prune_until_equal(diagnosed, "Callback_called_newv_notcalled_oldv: " + call, "Callback_notcalled_newv_called_oldv: " + call)
	diagnosed = [d if d is None else d.split(":")[0] for d in list(set(diagnosed))] # remove duplicates
	# remove noise from extra output if the diff is one that causes many lines of terminal spam
	# also, remove the function names (these were just here to distinguish them in the duplicate removal stage)
	diagnosed = [d for d in remove_noise_diffs(diagnosed) if d is not None]
	# heuristic: if the only difference is a return value, it's always been a function that just naturally has a different
	# return (for example, mkdirtemp)
	diagnosed = [] if diagnosed == [ "Diff_return_value" ] else diagnosed
	return( diagnosed) 

argparser = argparse.ArgumentParser(description="Diff analysis for ALT")
argparser.add_argument("--commit_list_file", metavar="commit_list_file", type=str, nargs='?', help="list of commits to compute diffs for")
argparser.add_argument("--commit_pair", metavar="commit_pair", type=str, nargs='?', help="specific pair of commits to compute diffs for")
argparser.add_argument("--libname", metavar="libname", type=str, help="library name")
argparser.add_argument("--numiters", metavar="numiters", type=int, help="number of iterations (for each test)")
argparser.add_argument("--outputfile", metavar="outputfile", type=str, nargs='?', help="file to output to")
argparser.add_argument("--diagnose_diffs", metavar="diagnose_diffs", type=bool, nargs='?', help="diagnose the diffs? true or false")
argparser.add_argument("--data_dir", metavar="data_dir", type=str, nargs="?", help="directory where the data files (testlog and fswatch) are")
argparser.add_argument("--diagnosed_diff_outfile", metavar="diagnosed_diff_outfile", type=str, nargs="?", help="file to output diff metadata to")
args = argparser.parse_args()

data_dir = args.data_dir if args.data_dir else "."

fswatch_prefix = data_dir + "/fswatch_test" + args.libname
log_prefix = data_dir + "/testlog_test" + args.libname 
out_printer = None
orig_stdout = sys.stdout
#orig_print = print
if args.outputfile:
	out_printer = open(args.outputfile, 'w')
	sys.stdout = out_printer
	#print = out_printer.write
commits = []
if args.commit_list_file:
	with open(args.commit_list_file) as f:
		commits = f.read().split()
elif args.commit_pair:
	commits = args.commit_pair.split("_")

# forward iterate through the commits
#commits = commits[0:3]
diff_map = {}
while len(commits) > 1:
	comp_commit = commits[0]
	cur_commit = commits[1]
	commits.remove(comp_commit)
	cur_diff_map = { "min_diff": None, "all_diffs": []}
	print("\nComparing commit: " + cur_commit + " to " + comp_commit)
	nodiff_log = False
	nodiff_watch = False
	min_diff_list = None
	for cur_commit_iter in range( args.numiters):
		if nodiff_log and nodiff_watch:
			continue
		cur_commit_watch_filename = fswatch_prefix + "_" + cur_commit + "_" + cur_commit + "_" + str(cur_commit_iter) + ".log"
		cur_commit_log_filename = log_prefix + "_" + cur_commit + "_" + cur_commit + "_" + str(cur_commit_iter) + ".log"
		for comp_commit_iter in range( args.numiters):
			comp_commit_watch_filename = fswatch_prefix + "_" + cur_commit + "_" + comp_commit + "_" + str(comp_commit_iter) + ".log"
			comp_commit_log_filename = log_prefix + "_" + cur_commit + "_" + comp_commit + "_" + str(comp_commit_iter) + ".log"
			if not nodiff_log:	
				(difflog_err, difflog_out, difflog_retcode) = run_command( "diff " + comp_commit_log_filename + " " + cur_commit_log_filename)
				if difflog_retcode == 0:
					print("\nno difference: " + cur_commit_log_filename + " --- " + comp_commit_log_filename)
					nodiff_log = True
					min_diff_list = []
				elif args.diagnose_diffs:
					cur_diff_list = diagnose_all_diffs( difflog_out.decode('utf-8'))
					min_diff_list = cur_diff_list if not min_diff_list or len(cur_diff_list) < len(min_diff_list) else min_diff_list
					cur_diff_map["all_diffs"] += [cur_diff_list]
					if len( cur_diff_list) == 0:
						print("\nno difference: " + cur_commit_log_filename + " --- " + comp_commit_log_filename)
						nodiff_log = True
						min_diff_list = []
			if not nodiff_watch:
				(diffwatch_err, diffwatch_out, diffwatch_retcode) = run_command( "diff " + comp_commit_watch_filename + " " + cur_commit_watch_filename)
				if diffwatch_retcode == 0:
					print("\nno difference: " + cur_commit_watch_filename + " --- " + comp_commit_watch_filename)
					nodiff_watch = True
	if nodiff_log and nodiff_watch:
		print("\nSame behaviour between commits " + cur_commit + " and commit " + comp_commit)
	else:
		print("\nBehavioural diff between commit " + cur_commit + " and commit " + comp_commit)
	if args.diagnose_diffs:
		print("\nMin diff:")
		print(min_diff_list)
	cur_diff_map["min_diff"] = min_diff_list
	diff_map[cur_commit + "_" + comp_commit] = cur_diff_map

if out_printer:
	sys.stdout = orig_stdout
	out_printer.close()
#	print = orig_print

if args.diagnosed_diff_outfile:
	with open(args.diagnosed_diff_outfile, 'w') as outf:
		outf.write(json.dumps(diff_map, indent=4))
else:
	# print total number of diffs
	total_diffs = 0
	min_diffs = []
	for key in diff_map.keys():
		min_diffs = min_diffs + diff_map[key]["min_diff"]
	total_diffs = len(min_diffs)
	print(min_diffs)