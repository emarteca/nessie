#!/bin/bash

#usage: ./seq_eval_cov_modes.sh num_tests lib_name lib_src_dir test_dir num_repeats

# finds the absolute path from relative path
# called realpath bc thats the utility on linux
# https://stackoverflow.com/questions/3572030/bash-script-absolute-path-with-os-x
realpathMACHACK() {
    [[ $1 = /* ]] && echo "$1" || echo "$PWD/${1#./}"
}

num_tests=$1
lib_name=$2
lib_src_dir=`realpathMACHACK $3`
test_dir=`realpathMACHACK $4`
repeats=$5

cur_dir=`pwd`

declare -a modes=("OGNessie" "TrackPrimitives" "MergeDiscGen" "ChainedMethods")

for repeat in $(seq 1 $repeats); do
	for mode in "${modes[@]}"
	do
		echo "Lib: "$lib_name" -- mode: "$mode" -- rep: " $repeat
		cd ..
		echo "Generating tests"
		cargo run -- --lib-name $lib_name --num-tests $num_tests --lib-src-dir $lib_src_dir --test-gen-mode=$mode >/dev/null 2>&1
		cd $cur_dir
		echo "Getting sequential coverage"
		./get_seq_coverage.sh $num_tests $test_dir $test_dir/toy_fs_dir > ${mode}_${num_tests}_${lib_name}_rep${repeat}
		rm ../js_tools/${lib_name}_discovery_${mode}.json
	done
done
