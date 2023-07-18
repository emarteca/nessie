#!/bin/bash

# regression testing
# given a list of commits for a repo, generate tests 
# for one commit and then run them on this and the next commit
# this needs the test generator to be in full output mode (to
# be able to diagnose output differences with a `diff`)
# the summary of the diffs is printed to a file

# this is a lot of copy paste from the gen_cov_for_repo

repo_link=$1
commit_list_file=$2
num_tests=$3
num_reps=$4
test_gen_mode=$5
mined_data_file=$6

cur_dir=`pwd`
TIMEOUT_SECONDS=30

# args: 
# 1: commit to checkout
checkoutAndSetup() { 
	commit_hash=$1
	
    rm -r node_modules 2>/dev/null # delete the node_modules if it's there; we want to re-setup the project
    # in case there's any changes (like to the package-lock.json) that prevent auto checkout, use -f
    git checkout -f $commit_hash > /dev/null

    # setup the project
    if [ -f "yarn.lock" ]; then
        yarn > /dev/null 
    else 
        npm install > /dev/null
    fi
    # note: if there's a custom build for the module, you may need to edit this
    npm run compile --if-present > /dev/null
    npm run build --if-present > /dev/null 
}

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

# read commits in from file into an array
readarray -t commits < $commit_list_file
readarray -td' ' commits <<<"$commits"
i=0
num_commit_pairs=$(( ${#commits[@]} - 1 ))
while (( $i < $num_commit_pairs )); do
    # get rid of newline 
    commit=`echo ${commits[$i]} | sed -r 's!\n!!g'`
    (( i = i + 1 ))
	next_commit=`echo ${commits[$i]} | sed -r 's!\n!!g'`

    echo "Checking out repo at commit: " $next_commit
    
    cd $test_dir
    checkoutAndSetup $next_commit 
    # now, move back to our source dir and run the test generator
    cd $cur_dir

	cur_test_dir=${test_output_dir}/tests_commit_${commit}
    cur_output_file=${test_output_dir}/${lib_name}_${num_tests}_${commit}_${nextCommit}_seqDiffs.out

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

	echo "--- Running tests for commit: " $commit
	# run the test suite, capture the output
	timeout $TIMEOUT_SECONDS mocha $cur_test_dir/metatest.js > ${test_output_dir}/testlog_test${lib_name}_${next_commit}_${commit}.log

    echo "--- Next commit: checking out repo at commit: " $commit
    
    cd $test_dir
    checkoutAndSetup $next_commit 
    # now, move back to our source dir and run the tests again
    cd $cur_dir

    echo "--- Running tests for commit: " $commit

	# run the test suite, capture the output
	timeout $TIMEOUT_SECONDS mocha $cur_test_dir/metatest.js > ${test_output_dir}/testlog_test${lib_name}_${next_commit}_${next_commit}.log

    python3 diff_analysis.py --commit_pair ${commit}_${next_commit} --libname $lib_name --numiters $num_reps --diagnose_diffs True --data_dir ${test_output_dir} --outputfile temp.json >> $cur_output_file

done
