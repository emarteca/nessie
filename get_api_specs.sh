#!/bin/bash

# usage: 	./get_api_specs.sh [api-to-analyze]
# example:	./get_api_specs.sh fs-extra

api_name=$1

cd js_tools

npm install $api_name
node api_info.js $api_name

cd ..