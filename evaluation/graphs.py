import pandas as pd
import numpy as np
import matplotlib.pyplot as plt

def plot_coverage_testgenmodes(lib_name, num_tests, num_reps, show_full_yrange=False):
	modes = ["OGNessie", "TrackPrimitives", "MergeDiscGen", "ChainedMethods"]

	for mode in modes:
		reps_data = []
		for rep in range(1, num_reps + 1):
			filename = mode + "_" + str(num_tests) + "_" + lib_name + "_rep" + str(rep)
			with open(filename) as f:
				# skip empty lines
				new_data = [float(x) for x in f.read().split("\n") if len(x) > 0]
				if len(new_data) != num_tests:
					print("UH OH: rep " + filename + " has len " + str(len(new_data)) + "; expected len " + str(num_tests))
					continue
				reps_data += [new_data]
		reps_data = np.array(reps_data).transpose()
		means = np.mean(reps_data, axis=1)
		plt.plot(means, label=mode)
		# no stdevs if there is only one run
		if len(reps_data) > 1:
			stdevs = np.std(reps_data, axis=1)
			plt.fill_between(range(0, num_tests), means+stdevs, means-stdevs, alpha=0.2, linewidth=0.5)
	plt.legend()
	plt.xlabel("Tests")
	plt.ylabel("Stmt coverage (%)")
	if show_full_yrange:
		plt.ylim([0, 1])
	plt.title("Cumulative % stmt coverage for " + lib_name + " over " + str(num_tests) + " tests (avg. " + str(num_reps) + " runs)")
	plt.show()


def main():
	# lib_name = "memfs"
	# num_tests = 500
	# num_reps = 4

	# lib_name = "fsextra"
	# num_tests = 500
	# num_reps = 7

	# lib_name = "q"
	# num_tests = 500
	# num_reps = 7

	lib_name = "zipafolder"
	num_tests = 500
	num_reps = 10

	plot_coverage_testgenmodes(lib_name, num_tests, num_reps)

main()
