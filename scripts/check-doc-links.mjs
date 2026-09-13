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
 * Parses a line to check if it is a valid opening code fence per CommonMark.
 */
function parseOpeningFence(line) {
  // CommonMark: 0-3 leading spaces, followed by 3+ backticks or 3+ tildes, followed by info string
  const match = line.match(/^ {0,3}(`{3,}|~{3,})(.*)$/);
  if (!match) return null;

  const fenceStr = match[1];
  const char = fenceStr[0];
  const len = fenceStr.length;
  const rawInfo = match[2];

  if (char === '`' && rawInfo.includes('`')) {
    return null;
  }

  const trimmedInfo = rawInfo.trim();
  const lang = trimmedInfo.split(/\s+/)[0] || '';

  return { char, len, lang, rawInfo, trimmedInfo };
}

/**
 * Computes unescaped remaining closing parens ')' from each position to the end of str.
 */
export function computeRemainingClosingParens(str) {
  const len = str.length;
  const remaining = new Int32Array(len + 1);
  for (let j = len - 1; j >= 0; j--) {
    let isClosingParen = false;
    if (str[j] === ')') {
      let backslashes = 0;
      let k = j - 1;
      while (k >= 0 && str[k] === '\\') {
        backslashes++;
        k--;
      }
      if (backslashes % 2 === 0) {
        isClosingParen = true;
      }
    }
    remaining[j] = remaining[j + 1] + (isClosingParen ? 1 : 0);
  }
  return remaining;
}

/**
 * Computes unescaped remaining closing angle brackets '>' from each position to the end of str.
 */
export function computeRemainingClosingAngles(str) {
  const len = str.length;
  const remaining = new Int32Array(len + 1);
  for (let j = len - 1; j >= 0; j--) {
    let isClosingAngle = false;
    if (str[j] === '>') {
      let backslashes = 0;
      let k = j - 1;
      while (k >= 0 && str[k] === '\\') {
        backslashes++;
        k--;
      }
      if (backslashes % 2 === 0) {
        isClosingAngle = true;
      }
    }
    remaining[j] = remaining[j + 1] + (isClosingAngle ? 1 : 0);
  }
  return remaining;
}

/**
 * Parses an angle-bracket destination starting with '<' at startPos.
 * CommonMark specification:
 * An angle-bracket destination starts with '<', ends with unescaped '>',
 * does not contain unescaped '<' or line breaks, and allows backslash escapes.
 *
 * Returns { destination, rawTarget, nextPos } or null if invalid/unclosed.
 */
export function parseAngleDestination(str, startPos = 0, remainingAngles = null, stats = null) {
  if (stats) stats.work++;
  if (startPos >= str.length || str[startPos] !== '<') {
    return null;
  }
  if (remainingAngles && remainingAngles[startPos + 1] === 0) {
    return null;
  }
  let p = startPos + 1;
  while (p < str.length) {
    if (stats) stats.work++;
    if (str[p] === '\\') {
      p += 2;
      continue;
    }
    if (str[p] === '<' || str[p] === '\n') {
      return null;
    }
    if (str[p] === '>') {
      return {
        destination: str.slice(startPos + 1, p),
        rawTarget: str.slice(startPos, p + 1),
        nextPos: p + 1,
      };
    }
    p++;
  }
  return null;
}

/**
 * Parses a bare destination (not starting with '<') at startPos.
 * In an inline link, bare destination terminates at:
 * - unescaped whitespace (which may precede a title or ')'), OR
 * - unescaped ')' when parenDepth is 0 (which ends the inline link).
 *
 * Returns { destination, rawTarget, nextPos } or null if invalid/unclosed.
 */
export function parseBareDestination(str, startPos = 0, remainingParens = null, stats = null) {
  if (stats) stats.work++;
  if (startPos >= str.length || str[startPos] === '<') {
    return null;
  }
  if (remainingParens && remainingParens[startPos] === 0) {
    return null;
  }
  let p = startPos;
  let parenDepth = 0;
  while (p < str.length) {
    if (stats) stats.work++;
    if (str[p] === '\\') {
      p += 2;
      continue;
    }
    if (str[p] === '(') {
      parenDepth++;
      if (remainingParens && parenDepth + 1 > remainingParens[p + 1]) {
        return null;
      }
      p++;
      continue;
    }
    if (str[p] === ')') {
      if (parenDepth > 0) {
        parenDepth--;
        p++;
        continue;
      }
      const dest = str.slice(startPos, p).trim();
      return {
        destination: dest,
        rawTarget: dest,
        nextPos: p,
      };
    }
    if (/\s/.test(str[p])) {
      if (parenDepth === 0) {
        const dest = str.slice(startPos, p).trim();
        return {
          destination: dest,
          rawTarget: dest,
          nextPos: p,
        };
      }
      return null;
    }
    p++;
  }
  if (parenDepth === 0) {
    const dest = str.slice(startPos, p).trim();
    return {
      destination: dest,
      rawTarget: dest,
      nextPos: p,
    };
  }
  return null;
}

/**
 * Parses an optional link title starting at startPos.
 * Returns { title, nextPos } or null if invalid/unclosed.
 * If no title opener is present, returns { title: null, nextPos: p } where p is advanced over whitespace.
 */
export function parseLinkTitle(str, startPos = 0, stats = null) {
  let p = startPos;
  while (p < str.length && /\s/.test(str[p])) {
    if (stats) stats.work++;
    p++;
  }
  if (p >= str.length) {
    return { title: null, nextPos: p };
  }
  const opener = str[p];
  if (opener !== '"' && opener !== "'" && opener !== '(') {
    return { title: null, nextPos: p };
  }
  const closer = opener === '(' ? ')' : opener;
  const titleStart = p + 1;
  p++;
  while (p < str.length) {
    if (stats) stats.work++;
    if (str[p] === '\\') {
      p += 2;
      continue;
    }
    if (str[p] === closer) {
      const title = str.slice(titleStart, p);
      p++;
      while (p < str.length && /\s/.test(str[p])) {
        if (stats) stats.work++;
        p++;
      }
      return { title, nextPos: p };
    }
    p++;
  }
  return null;
}

/**
 * Parses a destination starting at startPos, which may be angle-bracketed or bare.
 */
export function parseLinkDestination(str, startPos = 0, remainingParens = null, remainingAngles = null, stats = null) {
  if (startPos >= str.length) return null;
  if (str[startPos] === '<') {
    return parseAngleDestination(str, startPos, remainingAngles, stats);
  }
  return parseBareDestination(str, startPos, remainingParens, stats);
}

/**
 * Normalizes a raw link target string (from link.target) for file checking.
 */
export function normalizeLinkDestination(raw) {
  if (!raw) return '';
  const trimmed = raw.trim();
  if (trimmed.startsWith('<')) {
    const angle = parseAngleDestination(trimmed, 0);
    if (angle) {
      return angle.destination.trim();
    }
  }
  const bare = parseBareDestination(trimmed, 0);
  if (bare) {
    return bare.destination.trim();
  }
  return trimmed;
}

/**
 * Parses a reference definition line: [ref]: destination "title"
 */
function parseReferenceDefinition(line, lineNum) {
  const match = line.match(/^ {0,3}\[/);
  if (!match) return null;

  let i = match[0].length;
  const len = line.length;
  let labelEnd = -1;

  while (i < len) {
    if (line[i] === '\\') {
      i += 2;
      continue;
    }
    if (line[i] === ']') {
      labelEnd = i;
      break;
    }
    i++;
  }

  if (labelEnd === -1 || labelEnd + 1 >= len || line[labelEnd + 1] !== ':') {
    return null;
  }

  let destStart = labelEnd + 2;
  while (destStart < len && /\s/.test(line[destStart])) {
    destStart++;
  }
  if (destStart >= len) return null;

  const destRes = parseLinkDestination(line, destStart);
  if (!destRes) return null;

  const titleRes = parseLinkTitle(line, destRes.nextPos);
  if (!titleRes) return null;

  let tail = titleRes.nextPos;
  while (tail < len && /\s/.test(line[tail])) {
    tail++;
  }
  if (tail < len) {
    return null;
  }

  return {
    line: lineNum,
    target: destRes.rawTarget,
    destination: destRes.destination,
  };
}

/**
 * Parses markdown content and extracts local file links.
 */
export function extractLinks(filePath, content, options = {}) {
  const lines = content.split(/\r?\n/);
  const links = [];
  const stats = options && options.stats ? options.stats : null;
  let insideFence = false;
  let currentFenceChar = '';
  let currentFenceLen = 0;

  for (let lineNum = 0; lineNum < lines.length; lineNum++) {
    const line = lines[lineNum];

    if (insideFence) {
      const closeRegex = new RegExp(`^ {0,3}\\${currentFenceChar}{${currentFenceLen},}\\s*$`);
      if (closeRegex.test(line)) {
        insideFence = false;
        currentFenceChar = '';
        currentFenceLen = 0;
      }
      continue;
    }

    const openFence = parseOpeningFence(line);
    if (openFence) {
      insideFence = true;
      currentFenceChar = openFence.char;
      currentFenceLen = openFence.len;
      continue;
    }

    // Check for reference link definition: [ref]: destination "title"
    const refDef = parseReferenceDefinition(line, lineNum + 1);
    if (refDef) {
      links.push(refDef);
      continue;
    }

    const len = line.length;
    const remainingParens = computeRemainingClosingParens(line);
    const remainingAngles = computeRemainingClosingAngles(line);
    if (stats) stats.work += len * 2;

    // Single-pass forward scanner for inline links: [text](destination "optional title")
    let i = 0;
    let openBracketStack = [];

    while (i < len) {
      if (stats) stats.work++;
      if (line[i] === '\\') {
        i += 2;
        continue;
      }

      if (line[i] === '[') {
        openBracketStack.push(i);
        i++;
        continue;
      }

      if (line[i] === ']') {
        if (openBracketStack.length > 0) {
          const openBracket = openBracketStack.pop();
          if (i + 1 < len && line[i + 1] === '(') {
            let destStart = i + 2;
            while (destStart < len && /\s/.test(line[destStart])) {
              if (stats) stats.work++;
              destStart++;
            }
            if (destStart < len) {
              const destRes = parseLinkDestination(line, destStart, remainingParens, remainingAngles, stats);
              if (destRes !== null) {
                const titleRes = parseLinkTitle(line, destRes.nextPos, stats);
                if (titleRes !== null && titleRes.nextPos < len && line[titleRes.nextPos] === ')') {
                  links.push({
                    line: lineNum + 1,
                    target: destRes.rawTarget,
                    destination: destRes.destination,
                  });
                  i = titleRes.nextPos + 1;
                  openBracketStack = [];
                  continue;
                }
              }
            }
          }
        }
        i++;
        continue;
      }

      i++;
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
    let raw = link.destination !== undefined ? link.destination : normalizeLinkDestination(link.target);

    // Ignore external URLs (including protocol-relative URLs starting with //)
    if (/^(https?:|mailto:|ftp:|data:|\/\/)/i.test(raw)) {
      continue;
    }

    // Handle file:// URIs (e.g. file:///Users/... or file:///path/to/...)
    let targetPath = raw;
    if (targetPath.startsWith('file://')) {
      targetPath = targetPath.slice(7);
    }

    // Strip heading anchors / fragments (unescaped #)
    let fragmentIndex = -1;
    for (let j = 0; j < targetPath.length; j++) {
      if (targetPath[j] === '\\') {
        j++;
        continue;
      }
      if (targetPath[j] === '#') {
        fragmentIndex = j;
        break;
      }
    }
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

    // Unescape backslash escapes (e.g. \(, \), \<, \>, \\)
    targetPath = targetPath.replace(/\\(.)/g, '$1');

    let resolvedPath;
    if (path.isAbsolute(targetPath)) {
      if (targetPath.startsWith(repoRoot)) {
        resolvedPath = targetPath;
      } else if (fs.existsSync(targetPath)) {
        continue;
      } else {
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
