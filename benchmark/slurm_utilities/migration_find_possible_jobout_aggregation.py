import os, sys, subprocess


import subprocess, sys
qec_playground_root_dir = subprocess.run("git rev-parse --show-toplevel", cwd=os.path.dirname(__file__), shell=True, check=True, capture_output=True).stdout.decode(sys.stdout.encoding).strip(" \r\n")
process = subprocess.run(["git", "ls-files"], universal_newlines=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, cwd=qec_playground_root_dir)
output = process.stdout
files = output.split("\n")
print(f"number of files in the git repo: {len(files)}")

possible_aggregation_folders = dict()
possible_aggregation_number = 0
for filename in files:
    if filename[-7:] == ".jobout":
        folder = "/".join(filename.split("/")[:-1])
        if folder in possible_aggregation_folders:
            possible_aggregation_folders[folder] += 1
        else:
            possible_aggregation_folders[folder] = 1
        possible_aggregation_number += 1

print(f"number of files that can be reduced by this aggregation: {possible_aggregation_number}")

"""
2022/4/12
number of files in the git repo: 6694
number of files that can be reduced by this aggregation: 5264

we can reduce the number of files in this git repo by more than 80%!
"""

for folder in possible_aggregation_folders:
    print("SLURM_USE_EXISTING_DATA=1 python3", os.path.join(qec_playground_root_dir, folder, "..", "run_experiment.py"), ">/dev/null")
    # print("SLURM_USE_EXISTING_DATA=1 python3", os.path.join(qec_playground_root_dir, folder, "..", "run_experiment_long_scale.py"), ">/dev/null")


# SLURM_USE_EXISTING_DATA=1 python3 ...

for folder in possible_aggregation_folders:
    if os.path.exists(os.path.join(qec_playground_root_dir, folder, "_aggregated.hjson")):
        print("git rm --cached -r", os.path.join(qec_playground_root_dir, folder))


