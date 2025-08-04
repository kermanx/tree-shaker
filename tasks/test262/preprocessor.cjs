// @ts-check

const { treeShake } = require('@kermanx/tree-shaker')
const process = require('process');

const do_minify = true;

module.exports = function(test) {
  try {
    let prelude = test.contents.slice(0, test.insertionIndex);
    let main = test.contents.slice(test.insertionIndex);

    if (
      /\beval\b/.test(main)
      || /\bFunction\(/.test(main)
      || /\bevalScript\(/.test(main)
      || main.includes('$DONOTEVALUATE')
      || /\bwith\s*\(/.test(main)
      || /\busing\b/.test(main)
      || main.includes('noStrict')
      || main.includes('import-defer')
    ) {
      if (!process.stdout.isTTY) {
        console.log(`\n[SKIP] ${test.file}\n`)
      }
      return test;
    }

    process.stderr.write(`[TREESHAKE] ${test.file}\n`)

    let { output, diagnostics } = treeShake(main, "safest", do_minify);

    if (diagnostics.length) {
      throw new Error(diagnostics.join(' '));
    }

    test.contents = prelude + output;
  } catch (error) {
    test.result = {
      stderr: `${error.name}: ${error.message}\n`,
      stdout: '',
      error
    };
  }

  return test;
};
