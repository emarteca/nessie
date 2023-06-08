#!/bin/bash

# this script just gets the coverage of existing tests
# it's super brittle name and not meant to be reused

repo_link=$1
commit_hash=$2
num_tests=$3
num_reps=$4
test_gen_mode=$5
mined_data_file=$6

cur_dir=`pwd`
TIMEOUT_SECONDS=30

# [author name]_[project name] from the github repo link
# also get rid of - and . so we can reuse it as the variable name of the module import
lib_name=`echo $repo_link | sed -r 's!https://git(hub|lab).com/!!g' | sed -r 's!/!_!g' | sed -r 's!-!_!g' | sed -r 's!\.!!g'`

test_dir=TEST_REPO_${lib_name}
test_output_dir=${test_dir}_all_tests

# make a file to pipe all the coverage values to
touch ${test_output_dir}/coverage.csv

for rep in $(seq 1 $num_reps); do
	echo "Rep: " $rep

	cur_test_dir=${test_output_dir}/tests_rep_${rep}

	echo "--- Computing coverage"
	# compute the coverage, ignoring the test files
	timeout $TIMEOUT_SECONDS nyc --include=$test_dir/* --exclude=$cur_test_dir/**/*.js mocha $cur_test_dir/metatest.js > temp.out

	rel_out=`grep "All files" temp.out`
	python3 -c "
all_cov = '$rel_out'.split('|')
# coverage values are: 1: stmt; 2: branch; 3: fct
stmt_cov = 'NaN'
branch_cov = 'NaN'
if len(all_cov) > 2:
	stmt_cov = float(all_cov[1])
	branch_cov = float(all_cov[2])
print(str(stmt_cov) + ', ' + str(branch_cov))
" >> ${test_output_dir}/coverage.csv
done
