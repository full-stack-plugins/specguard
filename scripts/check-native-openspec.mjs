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
const hierarchyCases = [];
for (const depth of [4,5,6]) {
  for (const wrapper of ['plain','backtick','tilde','quote']) {
    for (const nested of [false,true]) {
      const heading = `${'#'.repeat(depth)} Scenario: Extra mandatory behavior\n- **THEN** the system rejects revoked credentials\n`;
      const fragment = wrapper === 'backtick' ? `\`\`\`markdown\n${heading}\`\`\`\n`
        : wrapper === 'tilde' ? `~~~markdown\n${heading}~~~\n`
        : wrapper === 'quote' ? heading.trimEnd().split('\n').map(line=>`> ${line}\n`).join('') : heading;
      const anchor = nested ? '#### Scenario: Incorrect password' : '#### Scenario: Correct password';
      const content = source.replace(anchor, `${fragment}\n${anchor}`);
      const report = await validator.validateSpecContent('session',content);
      const extraction = new MarkdownParser(content).parseSpec('session');
      const expected = wrapper === 'plain' && (!nested || depth === 4) ? 3 : 2;
      const scenarios = extraction.requirements[0].scenarios;
      if (!validator.isValid(report) || scenarios.length !== expected) throw new Error(`Official hierarchy case failed H${depth}/${wrapper}/${nested}: ${JSON.stringify({report,scenarios})}`);
      if (nested && expected === 2 && !scenarios[0].rawText.includes('Extra mandatory behavior')) throw new Error('Nested scenario text lost by official parser');
      hierarchyCases.push({depth,wrapper,nested,valid:report.valid,scenarios:scenarios.map(s=>s.name),specguardExpectation:expected===3?(depth>4?'unsupported hierarchy':'incomplete without extra registry ID'):'complete with nested/data text preserved'});
    }
  }
}
for (const depth of [4,5]) {
  const extra = `${'#'.repeat(depth)} Requirement: Extra mandatory requirement\nThe application MUST enforce revoked credentials.\n\n${'#'.repeat(depth+1)} Scenario: Revoked credential\n- **THEN** the system rejects access\n\n`;
  const content = source.replace('### Requirement: Password authentication', `${extra}### Requirement: Password authentication`);
  const report = await validator.validateSpecContent('session',content);
  const extraction = new MarkdownParser(content).parseSpec('session');
  if (!validator.isValid(report) || extraction.requirements.length !== 2) throw new Error(`Official skipped requirement case H${depth}: ${JSON.stringify({report,extraction})}`);
  hierarchyCases.push({depth,kind:'skipped-requirement',valid:report.valid,requirements:extraction.requirements.map(r=>r.name),specguardExpectation:'unsupported hierarchy'});
}
console.log(JSON.stringify({package:metadata.name, version:metadata.version, mainReport, changeReport, cases, hierarchyCases, requirement:parsed.requirements[0]},null,2));
