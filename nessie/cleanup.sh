#!/bin/bash

if [ -d js_tools/node_modules ]; then
	rm -r js_tools/node_modules
fi

if [ -f js_tools/package-lock.json ]; then
	rm js_tools/package-lock.json 2>/dev/null
fi
