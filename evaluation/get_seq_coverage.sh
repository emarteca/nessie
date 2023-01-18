#!/bin/bash

# usage: ./get_seq_coverage.sh num_tests test_dir [optional: toy_fs_dir]

# finds the absolute path from relative path
# called realpath bc thats the utility on linux
# https://stackoverflow.com/questions/3572030/bash-script-absolute-path-with-os-x
realpathMACHACK() {
    [[ $1 = /* ]] && echo "$1" || echo "$PWD/${1#./}"
}

num_tests=$1
test_dir=`realpathMACHACK $2`

cur_dir=`pwd`

coverage_command="nyc --report-dir $test_dir/coverage mocha $test_dir/metatest.js --grep "
cur_test_grep=""

# RESET FS ENV -- info
# optional: provide path to the toy filesystem dir -- if provided, it'll be copied now and then deleted 
# reset after each test run
toyFsCopyPath=""
toyFsOrigPath=""
if [ "$#" -ge 3 ]; then
        toyFsOrigPath=`realpathMACHACK $3`
        toyFsCopyPath=$cur_dir/"TOY_FS_TEMP_DIR"
        # delete the temp dir if it's already there
        rm -r $toyFsCopyPath 2>/dev/null
        mkdir $toyFsCopyPath
        cp -pr $toyFsOrigPath/* $toyFsCopyPath
fi

# get the list of all files in the test directory before running any tests
# (delete all files not in this list before every test run)
legitTestFilesList=$cur_dir/"KEEP_FILES_LIST"
ls $test_dir > $legitTestFilesList

# now, actually move into the testing dir and start running the tests
cd $test_dir

# delete the coverage dir if it's there from an earlier run
rm -r ../coverage 2>/dev/null

for i in $(seq 1 $num_tests); do
        cur_test_grep=`echo $cur_test_grep"test"$i"!"`
        $coverage_command \"$cur_test_grep\" > /dev/null 2>$1
        nyc report --reporter=json
        cd $cur_dir
        # echo "Tests: $i"
        node csv_from_coverage.js --input_file $test_dir/../coverage/coverage-final.json
        cd $test_dir
        cur_test_grep=`echo $cur_test_grep"|"`

        # RESET FS ENV
        # copy over the old toy fs dir if it's stored
        if [ -n "${toyFsOrigPath}" ]; then
                rm -r $toyFsOrigPath
                cp -pr $toyFsCopyPath $toyFsOrigPath
        fi
        # delete all the files not in the original list in the test dir 
        # (i.e., delete any new files made in the tests)
        for file in *; do
                if ! grep -qxFe "$file" $legitTestFilesList; then
                        rm -rf $file
                fi
        done
done

cd $cur_dir

