// Uses the independently installed official package. Does not execute source instructions.
import {readFile} from 'node:fs/promises';
import {resolve, dirname} from 'node:path';
import {fileURLToPath, pathToFileURL} from 'node:url';
const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const packageRoot = process.env.OPENSPEC_PACKAGE_ROOT ?? '/workspace/guard-docs-review/openspec-tool/node_modules/@fission-ai/openspec';
const metadata = JSON.parse(await readFile(resolve(packageRoot, 'package.json'), 'utf8'));
if (metadata.version !== '1.14.1') throw new Error(`Unverified OpenSpec ${metadata.version}`);
const {Validator} = await import(pathToFileURL(resolve(packageRoot, 'dist/core/validation/validator.js')));
const {MarkdownParser} = await import(pathToFileURL(resolve(packageRoot, 'dist/core/parsers/markdown-parser.js')));
const validator = new Validator(true);
const main = resolve(root, 'fixtures/source-versions/openspec-1.14.1/main/spec.md');
const mainReport = await validator.validateSpec(main);
const changeReport = await validator.validateChangeDeltaSpecs(resolve(root, 'fixtures/source-versions/openspec-1.14.1/change'));
if (!validator.isValid(mainReport) || !validator.isValid(changeReport)) throw new Error(JSON.stringify({mainReport, changeReport}));
const parsed = new MarkdownParser(await readFile(main,'utf8')).parseSpec('session');
if (parsed.requirements.length !== 1 || parsed.requirements[0].scenarios.length !== 2) throw new Error('Unexpected official parser extraction');
const golden = JSON.parse(await readFile(resolve(root,'fixtures/source-versions/openspec-1.14.1/official-extraction.json'),'utf8'));
if (JSON.stringify(parsed.requirements[0]) !== JSON.stringify(golden.requirement)) throw new Error('Official extraction changed from checked golden');
const source = await readFile(main,'utf8');
const variants = [
  ['closed-bom-crlf', '\ufeff'+source.replace('Password authentication\n','Password authentication ###\n').replace('Correct password\n','Correct password ####\n').replaceAll('\n','\r\n')],
  ['fenced-example', source.replace('#### Scenario: Correct password','```markdown\n### Requirement: Fake\n#### Scenario: Fake\n```\n\n#### Scenario: Correct password')],
];
const cases = [];
for (const [name, content] of variants) {
  const report = await validator.validateSpecContent('session',content);
  const extracted = new MarkdownParser(content).parseSpec('session');
  if (!validator.isValid(report) || JSON.stringify(extracted.requirements[0]) !== JSON.stringify(golden.requirement)) throw new Error(`Official parity failed: ${name}: ${JSON.stringify(report)}`);
  cases.push({name,report,extractionMatches:true});
}
console.log(JSON.stringify({package:metadata.name, version:metadata.version, mainReport, changeReport, cases, requirement:parsed.requirements[0]},null,2));
