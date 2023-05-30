# take the API call/signature mining output and parse it into a JSON file 
# this will then be used as seed input for the test generator

# format of the QL output: "acc path", "sig with types", "sig with (static) values"
# note: first line is also garbage (col name headers)
# example: "use (member join (member exports (module path)))",
#					"(_NOT_CONST_OR_FCT_,string)",
#					"(_NOT_CONST_OR_FCT_,
#					'jsonfile-tests-readfile-sync')"

# then, the output JSON will have the following format: list of pairs like this
# {
		# "pkg": pkg_name
		# "acc_path": acc path with use or def removed (all the APs start with one of these)
		# "sig_with_types": signature with the types of the statically available args
		# "sig_with_values": signature with the statically available args
# }

import sys
import json

def parse_line_into_json(line):
	comps = line.replace("\"\n", "").split("\",\"")
	ret = {}
	# first element is the AP: starts with "use or "def
	if comps[0][0:5] == "\"use " or comps[0][0:5] == "\"def ":
		ret["pkg"] = comps[0][5:].split("module ")[1].split(")")[0]
		ret["acc_path"] = comps[0][5:]
	else:
		print("UH OH: invalid pkg and acc_path from: " + comps[0])
		return(ret)
	ret["sig_with_types"] = comps[1]
	ret["sig_with_values"] = comps[2]
	return(ret)


def main():
	QL_results_file = sys.argv[1]
	res = []
	with open(QL_results_file) as f:
		next(f, None) # skip the header line
		for line in f:
			line_json = parse_line_into_json(line)
			res += [line_json]
	print(json.dumps(res, indent=4))

main()
