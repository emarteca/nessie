#!/bin/bash

# runs the gen_cov_for_repo.sh script over each line in the specified file
# the lines in the file are csv for repo_link, commit_hash

# usage: ./gen_cov_for_repo_list.sh file_with_list_of_repos_and_commits num_tests num_reps test_gen_mode [optional: mined_data_file]

repo_list_file=$1
num_tests=$2
num_reps=$3
test_gen_mode=$4
mined_data_file=$5

# dispatch the gen_cov_repo script for each repo link and commit hash in the specified file
while IFS=, read -r repo_link commit_hash; do 
	./gen_cov_for_repo.sh $repo_link $commit_hash $num_tests $num_reps $test_gen_mode $mined_data_file
done < $repo_list_file