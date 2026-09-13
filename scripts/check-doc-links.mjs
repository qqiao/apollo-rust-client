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
 * Builds precomputed escape-aware delimiter and candidate parse tables for a line.
 * Performs linear passes to answer destination and title queries in O(1):
 * - isEscaped[p]: 1 if character at p is escaped by an unescaped backslash, 0 otherwise.
 * - matchingParen[p]: index of matching unescaped ')' for unescaped '(' at p, or -1.
 * - nextWhitespace[p]: index of first whitespace >= p (or line length if none).
 * - nextNonWs[p]: index of first non-whitespace >= p (or line length if none).
 * - bareDestEnd[p]: end position of bare destination starting at p, or -1 if invalid.
 * - angleDestEnd[p]: end position of angle destination starting at p, or -1 if invalid.
 * - titleEnd[p]: end position of title starting at p, or -1 if invalid.
 */
export function buildLinkParseTables(line, stats = null) {
  const L = line.length;
  const isEscaped = new Uint8Array(L);
  const matchingParen = new Int32Array(L).fill(-1);
  const parenStack = [];
  let p = 0;

  while (p < L) {
    if (stats) stats.work++;
    if (line[p] === '\\') {
      if (p + 1 < L) {
        if (stats) stats.work++;
        isEscaped[p + 1] = 1;
      }
      p += 2;
    } else {
      if (line[p] === '(') {
        parenStack.push(p);
      } else if (line[p] === ')') {
        if (parenStack.length > 0) {
          const open = parenStack.pop();
          matchingParen[open] = p;
        }
      }
      p++;
    }
  }

  const nextWhitespace = new Int32Array(L + 1).fill(L);
  const nextNonWs = new Int32Array(L + 1).fill(L);
  const bareDestEnd = new Int32Array(L + 1).fill(-1);
  const angleDestEnd = new Int32Array(L + 1).fill(-1);
  const titleEnd = new Int32Array(L + 1).fill(-1);

  bareDestEnd[L] = L;

  let lastCloseAngle = -1;
  let lastCloseDouble = -1;
  let lastCloseSingle = -1;
  let lastCloseParen = -1;

  for (let j = L - 1; j >= 0; j--) {
    if (stats) stats.work++;
    const ch = line[j];
    const escaped = isEscaped[j] === 1;

    // Whitespace tracking
    if (/\s/.test(ch)) {
      nextWhitespace[j] = j;
      nextNonWs[j] = nextNonWs[j + 1];
    } else {
      nextWhitespace[j] = nextWhitespace[j + 1];
      nextNonWs[j] = j;
    }

    // Delimiter tracking
    if (!escaped) {
      if (ch === '>') {
        lastCloseAngle = j;
      } else if (ch === '<') {
        if (lastCloseAngle !== -1) {
          angleDestEnd[j] = lastCloseAngle + 1;
          lastCloseAngle = -1;
        } else {
          angleDestEnd[j] = -1;
        }
      }

      if (ch === '"') {
        if (lastCloseDouble !== -1) {
          titleEnd[j] = lastCloseDouble + 1;
        } else {
          titleEnd[j] = -1;
        }
        lastCloseDouble = j;
      } else if (ch === "'") {
        if (lastCloseSingle !== -1) {
          titleEnd[j] = lastCloseSingle + 1;
        } else {
          titleEnd[j] = -1;
        }
        lastCloseSingle = j;
      } else if (ch === ')') {
        lastCloseParen = j;
      } else if (ch === '(') {
        if (lastCloseParen !== -1) {
          titleEnd[j] = lastCloseParen + 1;
        } else {
          titleEnd[j] = -1;
        }
      }
    }

    // Bare destination end
    if (escaped) {
      bareDestEnd[j] = bareDestEnd[j + 1];
    } else if (ch === '\\') {
      bareDestEnd[j] = j + 2 <= L ? bareDestEnd[j + 2] : L;
    } else if (ch === ')' || /\s/.test(ch)) {
      bareDestEnd[j] = j;
    } else if (ch === '(') {
      const m = matchingParen[j];
      if (m === -1 || nextWhitespace[j] < m) {
        bareDestEnd[j] = -1;
      } else {
        bareDestEnd[j] = m + 1 <= L ? bareDestEnd[m + 1] : L;
      }
    } else {
      const code = line.charCodeAt(j);
      if ((code <= 0x1f || code === 0x7f) && !/\s/.test(ch)) {
        bareDestEnd[j] = -1;
      } else {
        bareDestEnd[j] = bareDestEnd[j + 1];
      }
    }
  }

  return {
    isEscaped,
    matchingParen,
    nextWhitespace,
    nextNonWs,
    bareDestEnd,
    angleDestEnd,
    titleEnd,
  };
}

/**
 * Parses an angle-bracket destination starting with '<' at startPos.
 * Returns { destination, rawTarget, nextPos } or null if invalid/unclosed.
 */
export function parseAngleDestination(str, startPos = 0, tables = null, stats = null) {
  if (stats) stats.work++;
  if (startPos >= str.length || str[startPos] !== '<') {
    return null;
  }
  const t = tables || buildLinkParseTables(str, stats);
  const destEnd = t.angleDestEnd[startPos];
  if (destEnd === -1) return null;
  return {
    destination: str.slice(startPos + 1, destEnd - 1),
    rawTarget: str.slice(startPos, destEnd),
    nextPos: destEnd,
  };
}

/**
 * Parses a bare destination (not starting with '<') at startPos.
 * Returns { destination, rawTarget, nextPos } or null if invalid/unclosed.
 */
export function parseBareDestination(str, startPos = 0, tables = null, stats = null) {
  if (stats) stats.work++;
  if (startPos >= str.length || str[startPos] === '<') {
    return null;
  }
  const t = tables || buildLinkParseTables(str, stats);
  const destEnd = t.bareDestEnd[startPos];
  if (destEnd === -1) return null;
  const rawTarget = str.slice(startPos, destEnd).trim();
  return {
    destination: rawTarget,
    rawTarget: rawTarget,
    nextPos: destEnd,
  };
}

/**
 * Parses an optional link title starting at startPos.
 * Returns { title, nextPos } or null if invalid/unclosed.
 * If no title opener is present, returns { title: null, nextPos: p } where p is advanced over whitespace.
 */
export function parseLinkTitle(str, startPos = 0, tables = null, stats = null) {
  if (stats) stats.work++;
  const t = tables || buildLinkParseTables(str, stats);
  const p = t.nextNonWs[startPos];
  if (p >= str.length) {
    return { title: null, nextPos: p };
  }
  const opener = str[p];
  if (opener !== '"' && opener !== "'" && opener !== '(') {
    return { title: null, nextPos: p };
  }
  const tEnd = t.titleEnd[p];
  if (tEnd === -1) {
    return null;
  }
  const title = str.slice(p + 1, tEnd - 1);
  const nextPos = t.nextNonWs[tEnd];
  return { title, nextPos };
}

/**
 * Parses a destination starting at startPos, which may be angle-bracketed or bare.
 */
export function parseLinkDestination(str, startPos = 0, tables = null, stats = null) {
  if (stats) stats.work++;
  if (startPos >= str.length) return null;
  if (str[startPos] === '<') {
    return parseAngleDestination(str, startPos, tables, stats);
  }
  return parseBareDestination(str, startPos, tables, stats);
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
function parseReferenceDefinition(line, lineNum, tables = null, stats = null) {
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

  const t = tables || buildLinkParseTables(line, stats);
  const destStart = t.nextNonWs[labelEnd + 2];
  if (destStart >= len) return null;

  const destRes = parseLinkDestination(line, destStart, t, stats);
  if (!destRes) return null;

  const titleRes = parseLinkTitle(line, destRes.nextPos, t, stats);
  if (!titleRes) return null;

  const tail = t.nextNonWs[titleRes.nextPos];
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

    // Precompute parse tables once for the line
    const tables = buildLinkParseTables(line, stats);

    // Check for reference link definition: [ref]: destination "title"
    const refDef = parseReferenceDefinition(line, lineNum + 1, tables, stats);
    if (refDef) {
      links.push(refDef);
      continue;
    }

    const len = line.length;

    // Single-pass forward scanner for inline links: [text](destination "optional title")
    let i = 0;
    let openBracketStack = [];

    while (i < len) {
      if (stats) stats.work++;
      if (tables.isEscaped[i]) {
        i++;
        continue;
      }
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
            const destStart = tables.nextNonWs[i + 2];
            if (destStart < len) {
              const destRes = parseLinkDestination(line, destStart, tables, stats);
              if (destRes !== null) {
                const titleRes = parseLinkTitle(line, destRes.nextPos, tables, stats);
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
