#!/bin/bash

for ql_outfile in ./mined_*.csv; do
	file_name_root=${ql_outfile%.csv} # get the filename minus the csv
	python parse_QL_into_json.py ${file_name_root}.csv > ${file_name_root}.json
	echo "Done " $file_name_root
done