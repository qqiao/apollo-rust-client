import test from 'node:test';
import assert from 'node:assert/strict';
import {
  extractSnippets,
  validateInventory,
  compileSnippets,
  REQUIRED_EXAMPLES,
} from '../../scripts/check-doc-examples.mjs';

test('extractSnippets extracts valid rust code blocks', () => {
  const markdown = `
# Title

<!-- apollo-example: test-snippet -->
\`\`\`rust
let x = 42;
\`\`\`
`;
  const snippets = extractSnippets('test.md', markdown);
  assert.equal(snippets.length, 1);
  assert.equal(snippets[0].id, 'test-snippet');
  assert.equal(snippets[0].code, 'let x = 42;');
});

test('extractSnippets rejects duplicate marker IDs in the same document', () => {
  const markdown = `
<!-- apollo-example: duplicate-id -->
\`\`\`rust
let x = 1;
\`\`\`

<!-- apollo-example: duplicate-id -->
\`\`\`rust
let y = 2;
\`\`\`
`;
  assert.throws(
    () => extractSnippets('dup.md', markdown),
    /Duplicate snippet ID 'duplicate-id'/
  );
});

test('extractSnippets rejects missing code fence after marker', () => {
  const markdown = `
<!-- apollo-example: no-fence -->
Some regular paragraph text without a code fence.
`;
  assert.throws(
    () => extractSnippets('no-fence.md', markdown),
    /Expected code fence after marker 'no-fence'/
  );
});

test('extractSnippets rejects non-rust language code block', () => {
  const markdown = `
<!-- apollo-example: wrong-lang -->
\`\`\`javascript
const a = 1;
\`\`\`
`;
  assert.throws(
    () => extractSnippets('wrong-lang.md', markdown),
    /specifies language 'javascript', expected 'rust'/
  );
});

test('extractSnippets rejects unclosed code fence', () => {
  const markdown = `
<!-- apollo-example: unclosed -->
\`\`\`rust
let a = 1;
// no closing backticks
`;
  assert.throws(
    () => extractSnippets('unclosed.md', markdown),
    /Unclosed code fence/
  );
});

test('extractSnippets rejects empty code block', () => {
  const markdown = `
<!-- apollo-example: empty -->
\`\`\`rust

\`\`\`
`;
  assert.throws(
    () => extractSnippets('empty.md', markdown),
    /Empty code snippet/
  );
});

test('validateInventory rejects missing required snippet', () => {
  const required = [
    { doc: 'docs/wiki/en/Configuration.md', id: 'nonexistent-snippet-id' },
  ];
  assert.throws(
    () => validateInventory(undefined, required),
    /Required example 'nonexistent-snippet-id' missing/
  );
});

test('compileSnippets rejects snippet with unavailable symbol in compiler failure', () => {
  const badSnippet = [
    {
      id: 'bad-symbol',
      filePath: 'test-bad.md',
      markerLine: 10,
      fenceLine: 11,
      code: 'let _ = apollo_rust_client::client_config::ClientConfig::nonexistent_symbol_call();',
    },
  ];

  assert.throws(
    () => compileSnippets(badSnippet),
    /Doc examples failed compilation under native-tls/
  );
});
