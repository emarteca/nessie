#!/bin/bash

# usage: 	./get_api_specs.sh [api-to-analyze] [OPTIONAL: dir-to-api-codebase]
# example:	./get_api_specs.sh fs-extra
# example:	./get_api_specs.sh jsonfile /path/to/jsonfile

api_name=$1
cur_dir=`pwd`

# if no path to api codebase specified
if [ -z $2 ]; then
	cd $cur_dir/js_tools
	npm install $api_name
else
	api_src_dir=$2
	cd $api_src_dir
	npm install
	cd $cur_dir/js_tools
fi
node api_info.js $api_name $2

cd $cur_dir
