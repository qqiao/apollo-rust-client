import fs from "node:fs";
import os from "node:os";
import path from "node:path";
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

test('compileSnippets rejects snippet with unavailable symbol in compiler failure and maps to source example (R3-4)', () => {
  const badSnippet = [
    {
      id: 'good-symbol',
      filePath: 'docs/wiki/en/good.md',
      markerLine: 5,
      fenceLine: 6,
      code: 'let _ = 1 + 1;',
    },
    {
      id: 'bad-symbol',
      filePath: 'docs/wiki/en/test-bad.md',
      markerLine: 42,
      fenceLine: 43,
      code: 'let _ = apollo_rust_client::client_config::ClientConfig::nonexistent_symbol_call();',
    },
  ];

  assert.throws(
    () => compileSnippets(badSnippet),
    (err) => {
      assert.match(err.message, /Doc examples failed compilation under native-tls/);
      assert.ok(err.message.includes('docs/wiki/en/test-bad.md'), 'Must report actual source file test-bad.md');
      assert.ok(err.message.includes('42'), 'Must report actual marker line 42');
      assert.ok(err.message.includes('bad-symbol'), 'Must report actual snippet ID bad-symbol');
      assert.ok(!err.message.includes('good-symbol'), 'Must not misattribute failure to good-symbol');
      assert.match(err.message, /nonexistent_symbol_call/, 'Must preserve compiler diagnostic details');
      return true;
    }
  );
});

test("extractSnippets ignores markers placed inside active code fences", () => {
  const markdown = `
\`\`\`rust
let a = 1;
<!-- apollo-example: inside-fence -->
\`\`\`rust
let b = 2;
\`\`\`
`;
  const snippets = extractSnippets("inside-fence.md", markdown);
  assert.equal(snippets.length, 0, "Markers inside code fences must not be extracted");
});

test("extractSnippets correctly tracks 4-backtick fence requiring 4 backticks to close", () => {
  const markdown = `
\`\`\`\`markdown
\`\`\`rust
let x = 1;
\`\`\`
<!-- apollo-example: nested-marker -->
\`\`\`\`

<!-- apollo-example: after-4-backticks -->
\`\`\`rust
let y = 2;
\`\`\`
`;
  const snippets = extractSnippets("nested.md", markdown);
  assert.equal(snippets.length, 1);
  assert.equal(snippets[0].id, "after-4-backticks");
});

test("extractSnippets correctly tracks tilde fences", () => {
  const markdown = `
~~~markdown
<!-- apollo-example: inside-tildes -->
~~~

<!-- apollo-example: after-tildes -->
\`\`\`rust
let z = 3;
\`\`\`
`;
  const snippets = extractSnippets("tildes.md", markdown);
  assert.equal(snippets.length, 1);
  assert.equal(snippets[0].id, "after-tildes");
});

test("validateInventory rejects hidden marker in active fence under both LF and CRLF, and accepts repaired form", () => {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "apollo-doc-inventory-"));
  const required = [{ doc: "example.md", id: "public-errors" }];
  const malformedLf = "```rust\nlet old = 1;\n<!-- apollo-example: public-errors -->\n```rust\nlet inner = 2;\n```\n";
  const malformedCrlf = malformedLf.replace(/\n/g, "\r\n");
  const repairedLf = "```rust\nlet old = 1;\n```\n\n<!-- apollo-example: public-errors -->\n```rust\nlet inner = 2;\n```\n";
  const repairedCrlf = repairedLf.replace(/\n/g, "\r\n");

  try {
    const docPath = path.join(tempDir, "example.md");

    // 1. Malformed under LF must reject
    fs.writeFileSync(docPath, malformedLf, "utf8");
    assert.throws(
      () => validateInventory(tempDir, required),
      /Required example 'public-errors' missing from example\.md/
    );

    // 2. Malformed under CRLF must reject
    fs.writeFileSync(docPath, malformedCrlf, "utf8");
    assert.throws(
      () => validateInventory(tempDir, required),
      /Required example 'public-errors' missing from example\.md/
    );

    // 3. Repaired under LF must pass
    fs.writeFileSync(docPath, repairedLf, "utf8");
    const snippetsLf = validateInventory(tempDir, required);
    assert.equal(snippetsLf.length, 1);
    assert.equal(snippetsLf[0].id, "public-errors");
    assert.equal(snippetsLf[0].code, "let inner = 2;");

    // 4. Repaired under CRLF must pass
    fs.writeFileSync(docPath, repairedCrlf, "utf8");
    const snippetsCrlf = validateInventory(tempDir, required);
    assert.equal(snippetsCrlf.length, 1);
    assert.equal(snippetsCrlf[0].id, "public-errors");
    assert.equal(snippetsCrlf[0].code, "let inner = 2;");
  } finally {
    fs.rmSync(tempDir, { recursive: true, force: true });
  }
});

test("extractSnippets rejects marked opener with invalid backtick info string before compilation", () => {
  const markdown = `
<!-- apollo-example: invalid-opener -->
\`\`\`rust\`
let x = 1;
\`\`\`
`;
  assert.throws(
    () => extractSnippets("invalid-opener.md", markdown),
    /Expected code fence after marker 'invalid-opener'/
  );
});
