const fs = require('fs');
const path = require('path');

let input = '';

let ignored = JSON.parse(fs.readFileSync(path.join(__dirname, 'ignored.json'), 'utf8'));
let v8Failed = fs.readFileSync(path.join(__dirname, 'v8_test262.status'), 'utf8').split(/\r?\n/).filter(Boolean).map(s => s + '.js');
let engine262Skip = fs.readFileSync(path.join(__dirname, 'engine262.skiplist'), 'utf8').split(/\r?\n/).filter(s => /^\w/.test(s));
let treeShakeSkipped = [];

process.stdin.on('data', chunk => {
  input += chunk;
});

process.stdin.on('end', () => {
  const lines = input
    .replace(/^Ran \d+ tests[\s\S]+/m, '')
    .replace(/^PASS.*$/gm, '')
    .replace(/^\[SKIP\](.*)$/gm, (_, p) => {
      treeShakeSkipped.push(p.trim().slice('test262/test/'.length))
      return ''
    })
    .replace(/^FAIL /gm, '')
    .replace(/ \(strict mode\)$/gm, '')
    .replace(/^.*\(default\)\n.*\n/gm, '')
    .split('\n')
    .filter(Boolean);

  const failedTests = {}
  let expectedFailedNum = 0;
  for (let i = 0; i < lines.length; i+=2) {
    let name = lines[i].slice('test262/test/'.length);
    if (ignored.includes(name) || v8Failed.includes(name) || engine262Skip.includes(name) || treeShakeSkipped.includes(name)) {
      expectedFailedNum++;
    } else {
      failedTests[name] = lines[i+1]?.replaceAll("`", "'") || '<NO OUTPUT>';
    }
  }

  fs.writeFileSync(path.join(__dirname, 'outputs.txt'), input);

  const failedList = Object.entries(failedTests).map(([name, message]) => {
    return `[${name}](https://github.com/tc39/test262/tree/main/test/${name}): \`${message.trim()}\``;
  }).join('\n');
  fs.writeFileSync(path.join(__dirname, 'failed.txt'), failedList);

  const stat = input.match(/^Ran \d+ tests[\s\S]+/m)[0];
  const total = +stat.match(/^Ran (\d+) tests$/m)[1];
  const passedNum = +stat.match(/^(\d+) passed$/m)[1];
  const failedNum = Object.keys(failedTests).length;
  const restMessage = stat.match(/^Treeshake[\s\S]+/m)?.[0] ?? "";
  fs.writeFileSync(path.join(__dirname, 'stat.txt'), `## Test262 Result

- **Failed: ${failedNum}**
- Total: ${total}
- Passed: ${passedNum}
- Ignored: ${expectedFailedNum}
${restMessage.split('\n').filter(Boolean).map(s => `- ${s.trim()}`).join('\n')}

${failedNum ? `
## Failed Tests

${failedList}
` : ''}`);
});
