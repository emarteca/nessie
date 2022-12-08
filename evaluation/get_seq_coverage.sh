#!/bin/bash

# usage: ./get_seq_coverage.sh num_tests test_dir

num_tests=$1
test_dir=$2

cur_dir=`pwd`
cd ..
#cd $test_dir

coverage_command="nyc --report-dir $test_dir/coverage mocha $test_dir/metatest.js --grep "
cur_test_grep=""

for i in $(seq 1 $num_tests); do
        cur_test_grep=`echo $cur_test_grep"test"$i"!"`
        $coverage_command \"$cur_test_grep\" > /dev/null 2>$1
        nyc report --reporter=json
        cd $cur_dir
        node csv_from_coverage.js --input_file ../$test_dir/coverage/coverage-final.json
        cd ..
	#cd $test_dir
        cur_test_grep=`echo $cur_test_grep"|"`
done

cd $cur_dir

