#!/bin/bash

# this script: generates num_tests tests for the specified repo at the specified commit
# then computes the coverage of these tests, and repeats the experiment num_reps times
# the output is: all the test suites, and the list of coverage values for each test suite
# note: this works for both github and gitlab repo link

# usage: ./gen_cov_for_repo.sh repo_link commit_hash num_tests num_reps test_gen_mode [optional: mined_data_file]

# notes: num_tests is the number of tests to generate
#		 num_reps is the number of repetitions to do of the entire testgen run

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

# get rid of the old testing dir if it's there already
if [ -d $test_output_dir ]; then
	rm -rf $test_output_dir
fi
mkdir $test_output_dir

# clone the project

if [ ! -d $test_dir ]; then
	mkdir $test_dir
	echo "Cloning repo: " $repo_link
	git clone $repo_link $test_dir
else
	echo "This repo has already been cloned, reusing source dir: " $test_dir
fi

cd $test_dir
echo "Checking out repo at commit: " $commit_hash
git checkout $commit_hash > /dev/null


# setup the project
if [ -f "yarn.lock" ]; then
	yarn > /dev/null 
else 
	npm install > /dev/null
fi
# note: if there's a custom build for the module, you may need to edit this
npm run build --if-present > /dev/null 

# now, move back to our source dir and run the test generator
cd $cur_dir

# make a file to pipe all the coverage values to
touch ${test_output_dir}/coverage.csv

for rep in $(seq 1 $num_reps); do
	echo "Rep: " $rep

	cur_test_dir=${test_output_dir}/tests_rep_${rep}
	mkdir $cur_test_dir

	echo "--- Generating tests"
	cargo run -- --lib-name $lib_name \
				 --num-tests $num_tests \
				 --lib-src-dir $test_dir \
				 --test-gen-mode=$test_gen_mode \
				 --redo-discovery \
				 --testing-dir=$cur_test_dir \
				 ${mined_data_file:+ --mined-call-data $mined_data_file} \
				 2> /dev/null

	echo "--- Computing coverage"
	# compute the coverage, ignoring the test files
	timeout $TIMEOUT_SECONDS nyc --include=$test_dir/* --exclude=$test_output_dir/**/*.js mocha $cur_test_dir/metatest.js > temp.out

	# extract the coverage
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

	rm temp.out
	
	# copy over the discovery file
	mv js_tools/${lib_name}_discovery_${test_gen_mode}.json $cur_test_dir
done

# at the end, copy over the api spec file (just the list of all functions the root of the module provides)
mv js_tools/${lib_name}_output.json $test_output_dir
