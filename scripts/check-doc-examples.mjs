#!/usr/bin/env node
/**
 * Verifies that canonical Rust documentation examples compile against the public crate.
 */

import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const REPO_ROOT = path.resolve(__dirname, '..');

export const REQUIRED_EXAMPLES = [
  {
    doc: 'docs/wiki/en/Configuration.md',
    id: 'native-config',
  },
  {
    doc: 'docs/wiki/en/Upgrade-Guide.md',
    id: 'public-imports',
  },
  {
    doc: 'docs/wiki/en/Error-Handling.md',
    id: 'public-errors',
  },
  {
    doc: 'docs/wiki/en/Configuration.md',
    id: 'environment-config',
  },
];

/**
 * Extracts marked code snippets from Markdown content.
 */
export function extractSnippets(filePath, content) {
  const lines = content.split('\n');
  const markerRegex = /<!--\s*apollo-example:\s*([\w-]+)\s*-->/;
  const snippets = [];
  const seenIds = new Set();

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const markerMatch = line.match(markerRegex);
    if (!markerMatch) continue;

    const id = markerMatch[1];
    const markerLine = i + 1;

    if (seenIds.has(id)) {
      throw new Error(`Duplicate snippet ID '${id}' in ${filePath} at line ${markerLine}`);
    }
    seenIds.add(id);

    // Look for fence opening immediately following the marker (allowing blank lines)
    let fenceLine = -1;
    let lang = '';
    let j = i + 1;
    while (j < lines.length) {
      const nextLine = lines[j].trim();
      if (nextLine === '') {
        j++;
        continue;
      }
      const fenceMatch = nextLine.match(/^```(\w+)?/);
      if (!fenceMatch) {
        throw new Error(
          `Expected code fence after marker '${id}' in ${filePath} at line ${j + 1}, found: '${lines[j]}'`
        );
      }
      fenceLine = j + 1;
      lang = fenceMatch[1] || '';
      break;
    }

    if (fenceLine === -1) {
      throw new Error(`Unterminated marker '${id}' in ${filePath} at line ${markerLine}: no code block found`);
    }

    if (lang !== 'rust') {
      throw new Error(
        `Snippet '${id}' in ${filePath} at line ${fenceLine} specifies language '${lang}', expected 'rust'`
      );
    }

    // Collect fence body until closing ```
    const codeLines = [];
    let closed = false;
    let k = j + 1;
    while (k < lines.length) {
      if (lines[k].trim() === '```') {
        closed = true;
        break;
      }
      codeLines.push(lines[k]);
      k++;
    }

    if (!closed) {
      throw new Error(
        `Unclosed code fence for snippet '${id}' starting at line ${fenceLine} in ${filePath}`
      );
    }

    const code = codeLines.join('\n').trim();
    if (code.length === 0) {
      throw new Error(`Empty code snippet '${id}' at line ${fenceLine} in ${filePath}`);
    }

    snippets.push({
      id,
      filePath,
      markerLine,
      fenceLine,
      code,
    });

    i = k; // continue scan after the closed fence
  }

  return snippets;
}

export function validateInventory(repoRoot = REPO_ROOT, required = REQUIRED_EXAMPLES) {
  const extractedByDoc = new Map();
  const allSnippets = [];

  // Group required by doc
  const requiredByDoc = new Map();
  for (const item of required) {
    if (!requiredByDoc.has(item.doc)) {
      requiredByDoc.set(item.doc, new Set());
    }
    requiredByDoc.get(item.doc).add(item.id);
  }

  for (const docRelPath of requiredByDoc.keys()) {
    const fullPath = path.resolve(repoRoot, docRelPath);
    if (!fs.existsSync(fullPath)) {
      throw new Error(`Required documentation file does not exist: ${docRelPath}`);
    }
    const content = fs.readFileSync(fullPath, 'utf8');
    const snippets = extractSnippets(docRelPath, content);
    extractedByDoc.set(docRelPath, snippets);
    allSnippets.push(...snippets);
  }

  // Verify all required IDs are present
  for (const item of required) {
    const docSnippets = extractedByDoc.get(item.doc) || [];
    const found = docSnippets.some((s) => s.id === item.id);
    if (!found) {
      throw new Error(
        `Required example '${item.id}' missing from ${item.doc}`
      );
    }
  }

  return allSnippets;
}

export function compileSnippets(snippets, repoRoot = REPO_ROOT) {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), 'apollo-doc-examples-'));
  const targetDir = path.resolve(repoRoot, 'target/doc-examples');
  fs.mkdirSync(targetDir, { recursive: true });

  try {
    const binDir = path.join(tempDir, 'src', 'bin');
    fs.mkdirSync(binDir, { recursive: true });

    const binConfigs = [];

    for (const snippet of snippets) {
      const safeName = snippet.id.replace(/-/g, '_');
      const binFileName = `example_${safeName}.rs`;
      const binFilePath = path.join(binDir, binFileName);

      const fileContent = `// Extracted from ${snippet.filePath}:${snippet.markerLine} (${snippet.id})
#![allow(unused_imports, unused_variables, dead_code)]

fn main() -> Result<(), Box<dyn std::error::Error>> {
${snippet.code}
    Ok(())
}
`;
      fs.writeFileSync(binFilePath, fileContent, 'utf8');

      binConfigs.push(`[[bin]]\nname = "example_${safeName}"\npath = "src/bin/${binFileName}"`);
    }

    const cargoToml = `[package]
name = "doc-examples-consumer"
version = "0.1.0"
edition = "2024"
publish = false

[dependencies.apollo-rust-client]
path = "${repoRoot}"
default-features = false

[features]
default = ["native-tls"]
native-tls = ["apollo-rust-client/native-tls"]
rustls = ["apollo-rust-client/rustls"]

${binConfigs.join('\n\n')}
`;

    fs.writeFileSync(path.join(tempDir, 'Cargo.toml'), cargoToml, 'utf8');

    // Seed temporary Cargo.lock if repo lockfile exists
    const repoLock = path.join(repoRoot, 'Cargo.lock');
    if (fs.existsSync(repoLock)) {
      fs.copyFileSync(repoLock, path.join(tempDir, 'Cargo.lock'));
    }

    console.log(`[check-doc-examples] Compiling ${snippets.length} examples under native-tls...`);
    const nativeRun = spawnSync(
      'cargo',
      ['clippy', '--manifest-path', path.join(tempDir, 'Cargo.toml'), '--all-targets', '--', '-D', 'warnings'],
      {
        cwd: repoRoot,
        env: { ...process.env, CARGO_TARGET_DIR: targetDir },
        stdio: 'pipe',
        encoding: 'utf8',
      }
    );

    if (nativeRun.status !== 0) {
      console.error(`[check-doc-examples] Clippy failed for native-tls:\n${nativeRun.stderr || nativeRun.stdout}`);
      throw new Error(`Doc examples failed compilation under native-tls`);
    }

    console.log(`[check-doc-examples] Compiling ${snippets.length} examples under rustls...`);
    const rustlsRun = spawnSync(
      'cargo',
      [
        'clippy',
        '--manifest-path',
        path.join(tempDir, 'Cargo.toml'),
        '--no-default-features',
        '--features',
        'rustls',
        '--all-targets',
        '--',
        '-D',
        'warnings',
      ],
      {
        cwd: repoRoot,
        env: { ...process.env, CARGO_TARGET_DIR: targetDir },
        stdio: 'pipe',
        encoding: 'utf8',
      }
    );

    if (rustlsRun.status !== 0) {
      console.error(`[check-doc-examples] Clippy failed for rustls:\n${rustlsRun.stderr || rustlsRun.stdout}`);
      throw new Error(`Doc examples failed compilation under rustls`);
    }

    console.log(`[check-doc-examples] All ${snippets.length} examples compiled successfully!`);
  } finally {
    fs.rmSync(tempDir, { recursive: true, force: true });
  }
}

// Direct execution entry point
if (process.argv[1] && path.resolve(process.argv[1]) === __filename) {
  try {
    const snippets = validateInventory();
    compileSnippets(snippets);
    process.exit(0);
  } catch (err) {
    console.error(`ERROR: ${err.message}`);
    process.exit(1);
  }
}
