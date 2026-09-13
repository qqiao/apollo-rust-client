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
