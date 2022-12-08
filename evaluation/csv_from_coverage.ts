import * as fs from 'fs';
import {argv} from 'yargs';

if (! argv.input_file) {
    console.log('Usage: ts-node csv_from_coverage.ts --input_file coverage_file.json');
    process.exit(1);
}

let input_file: string = argv.input_file;

const json = JSON.parse(fs.readFileSync(input_file, 'utf8'));

const codeFiles = Object.keys(json);

let statementSum = 0;
let results = {};
let sum = 0;

for (let i = 0; i < codeFiles.length; i++) {
    const statements = Object.values(json[codeFiles[i]].s);

    statementSum += statements.length;

    results[codeFiles[i]] = results[codeFiles[i]] ? results[codeFiles[i]] : []

    for (let j = 0; j < statements.length; j++) {
        if (<number>statements[j] > 0) {
            if (!results[codeFiles[i]].includes(j)) {
                results[codeFiles[i]].push(j)
            }
        }
    }
}

sum = statementSum
let sumCovered = 0
Object.values(results).forEach((array: Array<number>) => sumCovered += array.length)
console.log(sumCovered/sum)

