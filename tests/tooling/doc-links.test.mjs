import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import {
  extractLinks,
  checkFileLinks,
  runDocLinkCheck,
} from '../../scripts/check-doc-links.mjs';

test('extractLinks extracts inline and reference links outside code fences', () => {
  const markdown = `
# Links Test

Check [Configuration](../en/Configuration.md) for details.
Reference: [spec][spec-ref]

\`\`\`markdown
[fenced](do-not-extract.md)
\`\`\`

[spec-ref]: ../spec/README.md
`;
  const links = extractLinks('test.md', markdown);
  assert.equal(links.length, 2);
  assert.equal(links[0].target, '../en/Configuration.md');
  assert.equal(links[1].target, '../spec/README.md');
});

test('checkFileLinks ignores external URLs and fragment-only anchors', () => {
  const links = [
    { line: 1, target: 'https://example.com' },
    { line: 2, target: 'http://localhost:8080' },
    { line: 3, target: 'mailto:user@example.com' },
    { line: 4, target: '#heading-anchor' },
  ];
  const broken = checkFileLinks('/tmp/dummy.md', links);
  assert.equal(broken.length, 0);
});

test('checkFileLinks flags nonexistent relative target with file and line', () => {
  const links = [
    { line: 42, target: './nonexistent-file-xyz.md' },
  ];
  const broken = checkFileLinks('/tmp/dummy.md', links);
  assert.equal(broken.length, 1);
  assert.equal(broken[0].line, 42);
  assert.equal(broken[0].rawTarget, './nonexistent-file-xyz.md');
});

test('runDocLinkCheck detects broken links in a fixture directory', () => {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), 'apollo-doc-link-fixture-'));
  try {
    const docPath = path.join(tempDir, 'README.md');
    fs.writeFileSync(
      docPath,
      'See [broken target](./missing-subdoc.md) for info.\n',
      'utf8'
    );
    const result = runDocLinkCheck(tempDir);
    assert.equal(result.broken.length, 1);
    assert.equal(result.broken[0].filePath, docPath);
    assert.equal(result.broken[0].rawTarget, './missing-subdoc.md');
  } finally {
    fs.rmSync(tempDir, { recursive: true, force: true });
  }
});

test('extractLinks handles nested code fences without premature closure (F2 / AC-004)', () => {
  const markdown = [
    '# Nested Fence Test',
    '',
    '````markdown',
    '```rust',
    '[inner](do-not-extract-inner.md)',
    '```',
    '[still-fenced](do-not-extract-after-3.md)',
    '````',
    '',
    'Outside fence: [valid](../spec/README.md)',
  ].join('\n');

  const links = extractLinks('test.md', markdown);
  assert.equal(links.length, 1, `Expected only 1 link outside fence, got ${links.length}`);
  assert.equal(links[0].target, '../spec/README.md');
});

test('checkFileLinks parses angle-bracket destinations with spaces (F3 / AC-005)', () => {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), 'apollo-doc-link-spaces-'));
  try {
    const docsDir = path.join(tempDir, 'docs');
    fs.mkdirSync(docsDir, { recursive: true });
    const targetFile = path.join(docsDir, 'Getting Started.md');
    fs.writeFileSync(targetFile, '# Getting Started\n', 'utf8');

    const sourceFile = path.join(tempDir, 'README.md');
    const links = [
      { line: 1, target: '<docs/Getting Started.md>' },
      { line: 2, target: '<docs/Getting Started.md> "Guide Title"' },
    ];

    const broken = checkFileLinks(sourceFile, links, tempDir);
    assert.equal(broken.length, 0, `Expected 0 broken links for spaced angle-bracket targets, got: ${JSON.stringify(broken)}`);
  } finally {
    fs.rmSync(tempDir, { recursive: true, force: true });
  }
});

test('checkFileLinks ignores protocol-relative external URLs (F4 / AC-006)', () => {
  const links = [
    { line: 1, target: '//docs.example.com/guide' },
    { line: 2, target: '<//cdn.example.com/script.js>' },
    { line: 3, target: '//api.example.com/v1?token=test' },
  ];
  const broken = checkFileLinks('/tmp/dummy.md', links);
  assert.equal(broken.length, 0, `Expected 0 broken links for protocol-relative URLs, got: ${JSON.stringify(broken)}`);
});

test('extractLinks enforces CommonMark 0-3 space fence indentation and ignores 4-space indented fences', () => {
  const markdown = [
    '# Four Space Indented Fence Test',
    '',
    '    ```rust',
    '    [indented-link](target-1.md)',
    '    ```',
    '',
    '```markdown',
    '    ```',
    '[fenced-link](do-not-extract-inside-fence.md)',
    '```',
    '',
    '[outside-link](target-2.md)',
  ].join('\n');

  const links = extractLinks('test.md', markdown);
  // Line 4 link is in indented block (not CommonMark fence), so extractLinks extracts it.
  // Line 8 has 4 spaces, so it cannot close the fence at line 7! Line 9 is still inside fence!
  // Line 10 closes fence, so line 12 is extracted.
  const targets = links.map((l) => l.target);
  assert.ok(targets.includes('target-1.md'), 'Link inside 4-space indented block must be extracted');
  assert.ok(targets.includes('target-2.md'), 'Link outside fence must be extracted');
  assert.ok(!targets.includes('do-not-extract-inside-fence.md'), 'Link inside active fence must not be extracted prematurely by 4-space indented backticks');
});

test('extractLinks and checkFileLinks parse reference definitions with angle brackets and spaces', () => {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), 'apollo-doc-link-ref-'));
  try {
    const docsDir = path.join(tempDir, 'docs');
    fs.mkdirSync(docsDir, { recursive: true });
    fs.writeFileSync(path.join(docsDir, 'Getting Started.md'), '# Getting Started\n', 'utf8');

    const sourceFile = path.join(tempDir, 'README.md');
    const content = [
      '# Reference Links',
      '',
      '[guide]: <docs/Getting Started.md>',
      '[guide-title]: <docs/Getting Started.md> "Guide Title"',
      '',
      'See the [guide] for details.',
    ].join('\n');

    const links = extractLinks(sourceFile, content);
    assert.equal(links.length, 2, 'Expected 2 reference definition links extracted');
    assert.equal(links[0].target, '<docs/Getting Started.md>');
    assert.equal(links[1].target, '<docs/Getting Started.md>');

    const broken = checkFileLinks(sourceFile, links, tempDir);
    assert.equal(broken.length, 0, `Expected 0 broken links for reference definitions with angle brackets and spaces, got: ${JSON.stringify(broken)}`);
  } finally {
    fs.rmSync(tempDir, { recursive: true, force: true });
  }
});

test('extractLinks and checkFileLinks handle balanced inline destinations, escapes, titles, and unclosed inputs (R3-3)', () => {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), 'apollo-doc-link-r3-3-'));
  try {
    const docsDir = path.join(tempDir, 'docs');
    fs.mkdirSync(docsDir, { recursive: true });
    // Create actual files
    fs.writeFileSync(path.join(docsDir, 'guide_(v2).md'), '# Guide v2\n', 'utf8');
    fs.writeFileSync(path.join(docsDir, 'guide (v2).md'), '# Guide with space and parens\n', 'utf8');
    fs.writeFileSync(path.join(docsDir, 'regular.md'), '# Regular\n', 'utf8');

    const sourceFile = path.join(tempDir, 'README.md');
    const content = [
      '# Balanced and Escaped Inline Links',
      '',
      '- [v2 balanced](docs/guide_(v2).md)',
      '- [v2 escaped](docs/guide_\\(v2\\).md)',
      '- [v2 with title](docs/guide_(v2).md "Title with (parens)")',
      '- [v2 angle with space and parens](<docs/guide (v2).md>)',
      '- [v2 angle with title](<docs/guide (v2).md> "Title (v2)")',
      '- [regular](docs/regular.md)',
      '- [missing balanced](docs/nonexistent_(v1).md)',
      '',
      'Malformed/unclosed lines that must terminate without spurious truncated targets:',
      '- [unclosed paren](docs/guide_(v2).md',
      '- [unclosed angle](<docs/guide_(v2).md',
      '- [unclosed title](docs/guide_(v2).md "unclosed title)',
      '- [unclosed bracket(docs/guide_(v2).md)',
    ].join('\n');

    const links = extractLinks(sourceFile, content);
    const targets = links.map((l) => l.target);

    // Assert targets correctly extracted
    assert.ok(targets.includes('docs/guide_(v2).md'), 'Must extract balanced paren link intact without truncation');
    assert.ok(targets.includes('docs/guide_\\(v2\\).md'), 'Must extract escaped paren link intact');
    assert.ok(targets.includes('<docs/guide (v2).md>'), 'Must extract angle-delimited link with spaces and parens');
    assert.ok(targets.includes('docs/regular.md'), 'Must extract regular link');
    assert.ok(targets.includes('docs/nonexistent_(v1).md'), 'Must extract missing link target');

    // Assert unclosed/malformed inputs did not produce spurious partial targets
    assert.ok(!targets.some((t) => t.includes('unclosed')), 'Malformed lines must not emit spurious links');

    // Run checkFileLinks to verify existing files pass and missing files fail
    const broken = checkFileLinks(sourceFile, links, tempDir);
    assert.equal(broken.length, 1, `Expected exactly 1 broken link for nonexistent file, got: ${JSON.stringify(broken)}`);
    assert.ok(broken[0].rawTarget.includes('nonexistent_(v1).md'), 'The broken link must be the nonexistent file');
  } finally {
    fs.rmSync(tempDir, { recursive: true, force: true });
  }
});
