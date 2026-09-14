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

test('extractLinks and checkFileLinks handle escaped angle delimiters and titles in destinations', () => {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), 'apollo-doc-link-escaped-angle-'));
  try {
    const sourceFile = path.join(tempDir, 'README.md');
    // Create files with special delimiter characters in name
    fs.writeFileSync(path.join(tempDir, 'guide>v2.md'), '# Guide with > in name\n', 'utf8');
    fs.writeFileSync(path.join(tempDir, 'guide<v2.md'), '# Guide with < in name\n', 'utf8');

    const content = [
      '# Escaped Angle Test',
      '',
      '- [guide with >](<guide\\>v2.md>)',
      '- [guide with title](<guide\\>v2.md> "Title with > and \\> inside")',
      '- [guide with <](<guide\\<v2.md>)',
      '- [missing with >](<missing\\>v2.md>)',
      '',
      '[ref-guide]: <guide\\>v2.md> "Ref title with >"',
    ].join('\n');

    const links = extractLinks(sourceFile, content);
    assert.equal(links.length, 5, `Expected 5 links extracted, got: ${links.length}`);

    // End-to-end file check
    const broken = checkFileLinks(sourceFile, links, tempDir);
    assert.equal(broken.length, 1, `Expected only missing>v2.md to be broken, got: ${JSON.stringify(broken)}`);
    assert.equal(broken[0].rawTarget, '<missing\\>v2.md>');
    assert.ok(broken[0].resolvedPath.endsWith('missing>v2.md'), `Resolved path must end in missing>v2.md, got: ${broken[0].resolvedPath}`);

    // Synthetic link check (direct target without pre-parsed destination)
    const syntheticLinks = [
      { line: 10, target: '<guide\\>v2.md>' },
      { line: 11, target: '<guide\\>v2.md> "Title with >"' },
    ];
    const syntheticBroken = checkFileLinks(sourceFile, syntheticLinks, tempDir);
    assert.equal(syntheticBroken.length, 0, `Synthetic links with escaped angle must pass: ${JSON.stringify(syntheticBroken)}`);
  } finally {
    fs.rmSync(tempDir, { recursive: true, force: true });
  }
});

test('extractLinks performs linear-time scanning on adversarial unclosed/nested inputs without quadratic rescan', () => {
  // 1. Unclosed brackets of increasing lengths must exhibit linear (O(N)), sub-50ms behavior
  for (const n of [8000, 16000, 32000]) {
    const start = performance.now();
    const links = extractLinks('probe.md', '['.repeat(n));
    const elapsed = performance.now() - start;
    assert.equal(links.length, 0);
    assert.ok(elapsed < 100, `Scanning ${n} unclosed '[' took ${elapsed.toFixed(1)}ms, expected < 100ms`);
  }

  // 2. Unclosed closing brackets
  {
    const start = performance.now();
    const links = extractLinks('probe.md', ']'.repeat(32000));
    const elapsed = performance.now() - start;
    assert.equal(links.length, 0);
    assert.ok(elapsed < 100, `Scanning 32000 unclosed ']' took ${elapsed.toFixed(1)}ms`);
  }

  // 3. Repeated empty pairs
  {
    const start = performance.now();
    const links = extractLinks('probe.md', '[]'.repeat(16000));
    const elapsed = performance.now() - start;
    assert.equal(links.length, 0);
    assert.ok(elapsed < 100, `Scanning 16000 '[]' pairs took ${elapsed.toFixed(1)}ms`);
  }

  // 4. Opening brackets followed by closing brackets
  {
    const start = performance.now();
    const links = extractLinks('probe.md', '['.repeat(16000) + ']'.repeat(16000));
    const elapsed = performance.now() - start;
    assert.equal(links.length, 0);
    assert.ok(elapsed < 100, `Scanning 16000 '[' + 16000 ']' took ${elapsed.toFixed(1)}ms`);
  }

  // 5. Many opening brackets followed by a valid link
  {
    const start = performance.now();
    const links = extractLinks('probe.md', '['.repeat(16000) + '](valid.md)');
    const elapsed = performance.now() - start;
    assert.equal(links.length, 1);
    assert.equal(links[0].target, 'valid.md');
    assert.ok(elapsed < 100, `Scanning 16000 '[' followed by link took ${elapsed.toFixed(1)}ms`);
  }

  // 6. Unclosed angle destination after multiple brackets
  {
    const start = performance.now();
    const links = extractLinks('probe.md', '['.repeat(1000) + '](<unclosed');
    const elapsed = performance.now() - start;
    assert.equal(links.length, 0);
    assert.ok(elapsed < 100, `Scanning unclosed angle destination took ${elapsed.toFixed(1)}ms`);
  }
});

test('extractLinks and checkFileLinks handle trailing whitespace and tabs with bare and angle destinations, with and without titles', () => {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), 'apollo-doc-link-whitespace-'));
  try {
    const sourceFile = path.join(tempDir, 'README.md');

    // Create valid target files
    fs.writeFileSync(path.join(tempDir, 'existing-bare.md'), '# Existing Bare\n', 'utf8');
    fs.writeFileSync(path.join(tempDir, 'existing-angle.md'), '# Existing Angle\n', 'utf8');
    fs.writeFileSync(path.join(tempDir, 'existing-titled.md'), '# Existing Titled\n', 'utf8');
    fs.writeFileSync(path.join(tempDir, 'existing-tab.md'), '# Existing Tab\n', 'utf8');

    const content = [
      '# Trailing Whitespace Tests',
      '',
      '- [bare with space](missing-bare.md )',
      '- [bare existing](existing-bare.md )',
      '- [angle with space](<missing-angle.md> )',
      '- [angle existing](<existing-angle.md> )',
      '- [bare title space](missing-titled.md "title" )',
      '- [bare title existing](existing-titled.md "title" )',
      '- [angle title space](<missing-angle-titled.md> "title" )',
      '- [bare with tab](missing-tab.md\t)',
      '- [bare tab existing](existing-tab.md\t)',
      '- [angle with tab](<missing-tab.md>\t)',
      '- [bare tab title tab](missing-tab-titled.md\t"title"\t)',
      '- [angle tab title tab](<missing-angle-tab-titled.md>\t"title"\t)',
      '',
      '[ref-space]: missing-ref.md ',
      '[ref-title]: missing-ref-title.md "title" ',
    ].join('\n');

    const links = extractLinks(sourceFile, content);
    assert.equal(links.length, 14, `Expected 14 links extracted, got ${links.length}: ${JSON.stringify(links)}`);

    // Verify all targets and destinations extracted without truncation
    assert.ok(links.some((l) => l.destination === 'missing-bare.md'), 'missing-bare.md destination missing');
    assert.ok(links.some((l) => l.destination === 'existing-bare.md'), 'existing-bare.md destination missing');
    assert.ok(links.some((l) => l.destination === 'missing-angle.md'), 'missing-angle.md destination missing');
    assert.ok(links.some((l) => l.destination === 'existing-angle.md'), 'existing-angle.md destination missing');
    assert.ok(links.some((l) => l.destination === 'missing-titled.md'), 'missing-titled.md destination missing');
    assert.ok(links.some((l) => l.destination === 'existing-titled.md'), 'existing-titled.md destination missing');
    assert.ok(links.some((l) => l.destination === 'missing-tab.md'), 'missing-tab.md destination missing');
    assert.ok(links.some((l) => l.destination === 'existing-tab.md'), 'existing-tab.md destination missing');
    assert.ok(links.some((l) => l.destination === 'missing-ref.md'), 'missing-ref.md destination missing');
    assert.ok(links.some((l) => l.destination === 'missing-ref-title.md'), 'missing-ref-title.md destination missing');

    const broken = checkFileLinks(sourceFile, links, tempDir);
    // 14 links total: 4 exist, 10 are missing and must be flagged as broken
    assert.equal(broken.length, 10, `Expected exactly 10 broken links, got ${broken.length}: ${JSON.stringify(broken)}`);

    // Ensure none of the existing files were flagged
    const brokenDestinations = broken.map((b) => b.resolvedPath);
    assert.ok(!brokenDestinations.some((p) => p.endsWith('existing-bare.md')), 'existing-bare.md falsely flagged');
    assert.ok(!brokenDestinations.some((p) => p.endsWith('existing-angle.md')), 'existing-angle.md falsely flagged');
    assert.ok(!brokenDestinations.some((p) => p.endsWith('existing-titled.md')), 'existing-titled.md falsely flagged');
    assert.ok(!brokenDestinations.some((p) => p.endsWith('existing-tab.md')), 'existing-tab.md falsely flagged');
  } finally {
    fs.rmSync(tempDir, { recursive: true, force: true });
  }
});

test('extractLinks performs linear-time scanning with deterministic work accounting across repeated incomplete destinations', () => {
  // 1. Leader reproduction: repeated parens + space + closing parens
  for (const n of [1000, 2000, 4000, 8000]) {
    const stats = { work: 0 };
    const input = '[x]('.repeat(n) + ' ' + ')'.repeat(n);
    const start = performance.now();
    const links = extractLinks('probe.md', input, { stats });
    const elapsed = performance.now() - start;

    assert.equal(links.length, 1, `Expected 1 link (trailing empty destination), got ${links.length}`);
    assert.ok(elapsed < 100, `Scanning ${n} reproduction took ${elapsed.toFixed(1)}ms, expected < 100ms`);

    const workRatio = stats.work / input.length;
    assert.ok(
      workRatio <= 4.5,
      `Work ratio ${workRatio.toFixed(2)} (${stats.work} steps / ${input.length} chars) must be <= 4.5 for n=${n}`
    );
  }

  // 2. Leader reproduction followed by trailing valid link
  for (const n of [1000, 2000, 4000, 8000]) {
    const stats = { work: 0 };
    const input = '[x]('.repeat(n) + ' ' + ')'.repeat(n) + ' [valid](valid.md)';
    const start = performance.now();
    const links = extractLinks('probe.md', input, { stats });
    const elapsed = performance.now() - start;

    assert.equal(links.length, 2, `Expected 2 links, got ${links.length}`);
    assert.equal(links[1].destination, 'valid.md');
    assert.ok(elapsed < 100, `Scanning ${n} reproduction + valid took ${elapsed.toFixed(1)}ms`);

    const workRatio = stats.work / input.length;
    assert.ok(
      workRatio <= 4.5,
      `Work ratio ${workRatio.toFixed(2)} (${stats.work} steps / ${input.length} chars) must be <= 4.5 for n=${n}`
    );
  }

  // 3. Repeated malformed unclosed titles
  for (const n of [1000, 2000, 4000, 8000]) {
    const stats = { work: 0 };
    const input = '[x](dest "unclosed title '.repeat(n);
    const start = performance.now();
    const links = extractLinks('probe.md', input, { stats });
    const elapsed = performance.now() - start;

    assert.equal(links.length, 0, `Expected 0 links for unclosed titles, got ${links.length}`);
    assert.ok(elapsed < 100, `Scanning ${n} unclosed titles took ${elapsed.toFixed(1)}ms`);

    const workRatio = stats.work / input.length;
    assert.ok(
      workRatio <= 4.5,
      `Work ratio ${workRatio.toFixed(2)} (${stats.work} steps / ${input.length} chars) must be <= 4.5 for n=${n}`
    );
  }

  // 4. Repeated malformed unclosed titles followed by valid link
  for (const n of [1000, 2000, 4000, 8000]) {
    const stats = { work: 0 };
    const input = '[x](dest "unclosed title '.repeat(n) + ' [valid](valid.md "valid title")';
    const start = performance.now();
    const links = extractLinks('probe.md', input, { stats });
    const elapsed = performance.now() - start;

    assert.equal(links.length, 1, `Expected 1 valid link, got ${links.length}`);
    assert.equal(links[0].destination, 'valid.md');
    assert.ok(elapsed < 100, `Scanning ${n} unclosed titles + valid took ${elapsed.toFixed(1)}ms`);

    const workRatio = stats.work / input.length;
    assert.ok(
      workRatio <= 4.5,
      `Work ratio ${workRatio.toFixed(2)} (${stats.work} steps / ${input.length} chars) must be <= 4.5 for n=${n}`
    );
  }

  // 5. Repeated malformed unclosed angle destinations
  for (const n of [1000, 2000, 4000, 8000]) {
    const stats = { work: 0 };
    const input = '[x](<unclosed '.repeat(n);
    const start = performance.now();
    const links = extractLinks('probe.md', input, { stats });
    const elapsed = performance.now() - start;

    assert.equal(links.length, 0, `Expected 0 links for unclosed angles, got ${links.length}`);
    assert.ok(elapsed < 100, `Scanning ${n} unclosed angles took ${elapsed.toFixed(1)}ms`);

    const workRatio = stats.work / input.length;
    assert.ok(
      workRatio <= 4.5,
      `Work ratio ${workRatio.toFixed(2)} (${stats.work} steps / ${input.length} chars) must be <= 4.5 for n=${n}`
    );
  }

  // 6. Repeated malformed unclosed angle followed by valid link
  for (const n of [1000, 2000, 4000, 8000]) {
    const stats = { work: 0 };
    const input = '[x](<unclosed '.repeat(n) + ' [valid](<valid.md>)';
    const start = performance.now();
    const links = extractLinks('probe.md', input, { stats });
    const elapsed = performance.now() - start;

    assert.equal(links.length, 1, `Expected 1 valid link, got ${links.length}`);
    assert.equal(links[0].destination, 'valid.md');
    assert.ok(elapsed < 100, `Scanning ${n} unclosed angles + valid took ${elapsed.toFixed(1)}ms`);

    const workRatio = stats.work / input.length;
    assert.ok(
      workRatio <= 4.5,
      `Work ratio ${workRatio.toFixed(2)} (${stats.work} steps / ${input.length} chars) must be <= 4.5 for n=${n}`
    );
  }

  // 7. Repeated incomplete destinations: '[x]('.repeat(n)
  for (const n of [2000, 4000, 8000]) {
    const stats = { work: 0 };
    const input = '[x]('.repeat(n);
    const start = performance.now();
    const links = extractLinks('probe.md', input, { stats });
    const elapsed = performance.now() - start;

    assert.equal(links.length, 0, `Expected 0 links from incomplete destinations, got ${links.length}`);
    assert.ok(elapsed < 100, `Scanning ${n} incomplete '[x](' took ${elapsed.toFixed(1)}ms, expected < 100ms`);

    const workRatio = stats.work / input.length;
    assert.ok(
      workRatio <= 4.5,
      `Work ratio ${workRatio.toFixed(2)} (${stats.work} steps / ${input.length} chars) must be <= 4.5 for n=${n}`
    );
  }

  // 8. Repeated incomplete destinations followed by a valid link: '[x]('.repeat(n) + '[x](valid.md)'
  for (const n of [2000, 4000, 8000]) {
    const stats = { work: 0 };
    const input = '[x]('.repeat(n) + '[x](valid.md)';
    const start = performance.now();
    const links = extractLinks('probe.md', input, { stats });
    const elapsed = performance.now() - start;

    assert.equal(links.length, 1, `Expected exactly 1 valid link at tail, got ${links.length}`);
    assert.equal(links[0].destination, 'valid.md');
    assert.ok(elapsed < 100, `Scanning ${n} incomplete + valid took ${elapsed.toFixed(1)}ms, expected < 100ms`);

    const workRatio = stats.work / input.length;
    assert.ok(
      workRatio <= 4.5,
      `Work ratio ${workRatio.toFixed(2)} (${stats.work} steps / ${input.length} chars) must be <= 4.5 for n=${n}`
    );
  }

  // 9. Repeated incomplete angle destinations: '[x](<'.repeat(n)
  for (const n of [2000, 4000, 8000]) {
    const stats = { work: 0 };
    const input = '[x](<'.repeat(n);
    const start = performance.now();
    const links = extractLinks('probe.md', input, { stats });
    const elapsed = performance.now() - start;

    assert.equal(links.length, 0);
    assert.ok(elapsed < 100);
    const workRatio = stats.work / input.length;
    assert.ok(workRatio <= 4.5, `Work ratio ${workRatio.toFixed(2)} must be <= 4.5 for angle n=${n}`);
  }

  // 10. Repeated incomplete angle destinations followed by valid link: '[x](<'.repeat(n) + '[x](<valid.md>)'
  for (const n of [2000, 4000, 8000]) {
    const stats = { work: 0 };
    const input = '[x](<'.repeat(n) + '[x](<valid.md>)';
    const start = performance.now();
    const links = extractLinks('probe.md', input, { stats });
    const elapsed = performance.now() - start;

    assert.equal(links.length, 1);
    assert.equal(links[0].destination, 'valid.md');
    assert.ok(elapsed < 100);
    const workRatio = stats.work / input.length;
    assert.ok(workRatio <= 4.5, `Work ratio ${workRatio.toFixed(2)} must be <= 4.5 for angle valid n=${n}`);
  }
});

test('extractLinks ignores link syntax inside inline backtick code spans', () => {
  const markdown = [
    '# Inline Code Span Test',
    '',
    'See `[missing](not-a-file.md)` in backticks.',
    'Also double backtick: ``[another-missing](also-not-a-file.md)`` inside code.',
    'Valid link outside: [valid](../spec/README.md).',
  ].join('\n');

  const links = extractLinks('inline-code.md', markdown);
  assert.equal(links.length, 1, `Expected only 1 link outside inline code spans, got ${links.length}`);
  assert.equal(links[0].target, '../spec/README.md');
});
