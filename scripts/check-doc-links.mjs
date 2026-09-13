#!/usr/bin/env node
/**
 * Verifies local Markdown file links across repository documentation.
 * Heading anchors and external URLs are excluded from validation.
 */

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const REPO_ROOT = path.resolve(__dirname, '..');

const SCOPED_ROOT_FILES = ['README.md', 'README_zh.md', 'CHANGELOG.md'];
const SCOPED_DIRS = ['docs/wiki', 'spec'];

/**
 * Finds all markdown files within the defined scope.
 */
export function findMarkdownFiles(repoRoot = REPO_ROOT) {
  const files = [];

  for (const rootFile of SCOPED_ROOT_FILES) {
    const fullPath = path.resolve(repoRoot, rootFile);
    if (fs.existsSync(fullPath)) {
      files.push(fullPath);
    }
  }

  function walk(dir) {
    const entries = fs.readdirSync(dir, { withFileTypes: true });
    for (const entry of entries) {
      if (entry.name.startsWith('.') || entry.name === 'target' || entry.name === 'node_modules' || entry.name === 'pkg') {
        continue;
      }
      const fullPath = path.join(dir, entry.name);
      if (entry.isDirectory()) {
        walk(fullPath);
      } else if (entry.isFile() && entry.name.endsWith('.md')) {
        files.push(fullPath);
      }
    }
  }

  for (const dir of SCOPED_DIRS) {
    const fullDir = path.resolve(repoRoot, dir);
    if (fs.existsSync(fullDir)) {
      walk(fullDir);
    }
  }

  return files;
}

/**
 * Parses markdown content and extracts local file links.
 */
export function extractLinks(filePath, content) {
  const lines = content.split('\n');
  const links = [];
  let inCodeBlock = false;

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];

    // Toggle fenced code blocks
    if (/^\s*```/.test(line) || /^\s*~~~/.test(line)) {
      inCodeBlock = !inCodeBlock;
      continue;
    }

    if (inCodeBlock) {
      continue;
    }

    // Inline links: [text](target)
    const inlineRegex = /\[([^\]]*)\]\(([^)]+)\)/g;
    let match;
    while ((match = inlineRegex.exec(line)) !== null) {
      links.push({
        line: i + 1,
        target: match[2].trim(),
      });
    }

    // Reference definitions: [ref]: target
    const refRegex = /^\s*\[([^\]]+)\]:\s*(\S+)/;
    const refMatch = line.match(refRegex);
    if (refMatch) {
      links.push({
        line: i + 1,
        target: refMatch[2].trim(),
      });
    }
  }

  return links;
}

/**
 * Checks all extracted links for existence of target files.
 */
export function checkFileLinks(filePath, links, repoRoot = REPO_ROOT) {
  const broken = [];

  for (const link of links) {
    let raw = link.target;

    // Handle angle brackets e.g. <path>
    if (raw.startsWith('<') && raw.endsWith('>')) {
      raw = raw.slice(1, -1).trim();
    }

    // Strip title if present, e.g. [text](path "title")
    const spaceIndex = raw.indexOf(' ');
    if (spaceIndex !== -1) {
      raw = raw.slice(0, spaceIndex).trim();
    }

    // Ignore external URLs
    if (/^(https?:|mailto:|ftp:|data:)/i.test(raw)) {
      continue;
    }

    // Handle file:// URIs (e.g. file:///Users/... or file:///path/to/...)
    let targetPath = raw;
    if (targetPath.startsWith('file://')) {
      targetPath = targetPath.slice(7);
    }

    // Strip heading anchors / fragments
    const fragmentIndex = targetPath.indexOf('#');
    if (fragmentIndex !== -1) {
      targetPath = targetPath.slice(0, fragmentIndex);
    }

    // If only an anchor (e.g. #section), ignore (anchor checks out of scope)
    if (targetPath === '') {
      continue;
    }

    // URL-decode percent encoding (e.g. %20 -> space)
    try {
      targetPath = decodeURIComponent(targetPath);
    } catch {
      // ignore decode failure
    }

    let resolvedPath;
    if (path.isAbsolute(targetPath)) {
      // If absolute, see if it maps into repoRoot or is existing system path
      if (targetPath.startsWith(repoRoot)) {
        resolvedPath = targetPath;
      } else if (fs.existsSync(targetPath)) {
        continue;
      } else {
        // Assume absolute from repo root if leading slash
        resolvedPath = path.resolve(repoRoot, '.' + targetPath);
      }
    } else {
      resolvedPath = path.resolve(path.dirname(filePath), targetPath);
    }

    if (!fs.existsSync(resolvedPath)) {
      broken.push({
        filePath,
        line: link.line,
        rawTarget: link.target,
        resolvedPath,
      });
    }
  }

  return broken;
}

export function runDocLinkCheck(repoRoot = REPO_ROOT) {
  const files = findMarkdownFiles(repoRoot);
  const allBroken = [];
  let totalLinks = 0;

  for (const file of files) {
    const content = fs.readFileSync(file, 'utf8');
    const links = extractLinks(file, content);
    totalLinks += links.length;
    const broken = checkFileLinks(file, links, repoRoot);
    allBroken.push(...broken);
  }

  return { filesCount: files.length, totalLinks, broken: allBroken };
}

if (process.argv[1] && path.resolve(process.argv[1]) === __filename) {
  const { filesCount, totalLinks, broken } = runDocLinkCheck();
  console.log(`[check-doc-links] Checked ${totalLinks} links across ${filesCount} markdown files.`);

  if (broken.length > 0) {
    console.error(`[check-doc-links] Found ${broken.length} broken local file links:`);
    for (const item of broken) {
      const relFile = path.relative(REPO_ROOT, item.filePath);
      console.error(`  - ${relFile}:${item.line} -> '${item.rawTarget}' (target does not exist: ${item.resolvedPath})`);
    }
    process.exit(1);
  }

  console.log(`[check-doc-links] All local file links verified successfully!`);
  process.exit(0);
}
