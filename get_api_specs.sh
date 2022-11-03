#!/bin/bash

# usage: 	./get_api_specs.sh lib_name=<api-to-analyze> [lib_src_dir=<dir-to-api-codebase>] [import_code_file=<file-with-custom-import>]
# example:	./get_api_specs.sh lib_name=fs-extra
# example:	./get_api_specs.sh lib_name=jsonfile lib_src_dir=/path/to/jsonfile import_code_file=/path/to/file/with/import

# https://unix.stackexchange.com/questions/129391/passing-named-arguments-to-shell-scripts
for arg in "$@"; do
	arg_name=$(echo $arg | cut -f1 -d=)
	arg_name_len=${#arg_name}
	arg_val="${arg:$arg_name_len+1}"

	# need absolute paths here
	if [[ "$arg_name" == "lib_src_dir" ]]; then
		arg_val=`realpath $arg_val`
	fi

	if [[ "$arg_name" == "import_code_file" ]]; then
		arg_val=`realpath $arg_val`
	fi

	export "$arg_name"="$arg_val"
done

cur_dir=`pwd`

# if no path to api codebase specified
if [ -z $lib_src_dir ]; then
	cd $cur_dir/js_tools
	npm install $lib_name
else
	cd $lib_src_dir
	npm install
	cd $cur_dir/js_tools
fi
node api_info.js --lib_name=$lib_name --lib_src_dir=$lib_src_dir --import_code_file=$import_code_file

cd $cur_dir
