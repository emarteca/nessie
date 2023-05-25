// to compile: tsc csv_from_coverage.ts --downlevelIteration

import * as fs from 'fs';
import {argv} from 'yargs';

if (! argv.input_file) {
    console.log('Usage: ts-node csv_from_coverage.ts --input_file coverage_file.json [--include_branch_coverage]');
    process.exit(1);
}

let input_file: string = argv.input_file;
let include_branch_cov: boolean = argv.include_branch_coverage;

const json = JSON.parse(fs.readFileSync(input_file, 'utf8'));

const codeFiles = Object.keys(json);

let statementSum = 0;
let branchSum = 0;
let resultsStatements = {};
let resultsBranches = {}

let numStatCovered = 0;
let numBranchCovered = 0;

for (let i = 0; i < codeFiles.length; i++) {
    const statements = Object.values(json[codeFiles[i]].s);
    const branches = Object.values(json[codeFiles[i]].b);
    // console.log(branches)

    statementSum += statements.length;
    branchSum += branches.length;

    resultsStatements[codeFiles[i]] = resultsStatements[codeFiles[i]] ? resultsStatements[codeFiles[i]] : []
    resultsBranches[codeFiles[i]] = resultsBranches[codeFiles[i]] ? resultsBranches[codeFiles[i]].map(t => JSON.stringify(t)) : []

    for (let j = 0; j < statements.length; j++) {
        if (<number>statements[j] > 0) {
            if (!resultsStatements[codeFiles[i]].includes(j)) {
                numStatCovered++;
            }
        }
    }

    for (let j = 0; j < branches.length; j++) {
        if ((<Array<number>>branches[j]).reduce((a, b) => a + b, 0) > 0) {
            if (!resultsBranches[codeFiles[i]].includes(JSON.stringify(branches[j]))) {
                resultsBranches[codeFiles[i]].push(JSON.stringify(branches[j]))
                numBranchCovered++;
            }
        }
    }
    console.log(resultsBranches[codeFiles[i]])
}

console.log(numStatCovered/statementSum + (include_branch_cov ? ", " + numBranchCovered/branchSum : ""));

