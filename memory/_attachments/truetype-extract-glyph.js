#!/usr/bin/env node
/**
 * Extract one glyph's outline from a TrueType font as an SVG path — no deps.
 *
 * Usage:
 *   node truetype-extract-glyph.js "S" /path/to/Font.ttf [targetHeight] [viewBox]
 *   targetHeight: glyph cap-height in viewBox units (default 17.6)
 *   viewBox:      square canvas size, glyph centred (default 48)
 *
 * Gotchas baked in (each cost real debugging time):
 *  1. TrueType stores ALL x deltas for every point first, THEN all y deltas.
 *     Reading x/y per point (interleaved) yields a plausible-looking but wrong
 *     outline with no parse error.
 *  2. Short deltas are unsigned bytes; the flag bits (0x10 / 0x20) decide the
 *     sign. readInt8 on them accumulates ±256 errors past byte value 127.
 * Self-check: parsed bbox MUST equal the glyf header bbox — printed side by side.
 */
const fs = require("node:fs");

const CHAR = process.argv[2] || "S";
const TTF = process.argv[3];
const TARGET_H = Number(process.argv[4] || 17.6);
const VIEW = Number(process.argv[5] || 48);
if (!TTF) {
  console.error("usage: node truetype-extract-glyph.js <char> <font.ttf> [targetHeight] [viewBox]");
  process.exit(1);
}

const buf = fs.readFileSync(TTF);
const numTables = buf.readUInt16BE(4);
const tables = {};
for (let i = 0; i < numTables; i++) {
  const o = 12 + i * 16;
  tables[buf.toString("ascii", o, o + 4)] = { offset: buf.readUInt32BE(o + 8), length: buf.readUInt32BE(o + 12) };
}

const head = tables.head.offset;
const indexToLocFormat = buf.readInt16BE(head + 50);

const cmapBase = tables.cmap.offset;
const numSubtables = buf.readUInt16BE(cmapBase + 2);
let best = null;
for (let i = 0; i < numSubtables; i++) {
  const rec = cmapBase + 4 + i * 8;
  const platform = buf.readUInt16BE(rec);
  const encoding = buf.readUInt16BE(rec + 2);
  const offset = cmapBase + buf.readUInt32BE(rec + 4);
  const format = buf.readUInt16BE(offset);
  const score = (platform === 3 && encoding === 10 ? 3 : 0) + (platform === 3 && encoding === 1 ? 2 : 0) + (platform === 0 ? 1 : 0);
  if (!best || score > best.score) best = { score, offset, format };
}

function glyphIdFor(code) {
  const { offset, format } = best;
  if (format === 12) {
    const nGroups = buf.readUInt32BE(offset + 12);
    for (let g = 0; g < nGroups; g++) {
      const o = offset + 16 + g * 12;
      const start = buf.readUInt32BE(o);
      const end = buf.readUInt32BE(o + 4);
      if (code >= start && code <= end) return buf.readUInt32BE(o + 8) + (code - start);
    }
  } else if (format === 4) {
    const segCount = buf.readUInt16BE(offset + 6) / 2;
    const endBase = offset + 14;
    const startBase = endBase + segCount * 2 + 2;
    const deltaBase = startBase + segCount * 2;
    const rangeBase = deltaBase + segCount * 2;
    for (let s = 0; s < segCount; s++) {
      const end = buf.readUInt16BE(endBase + s * 2);
      const start = buf.readUInt16BE(startBase + s * 2);
      if (code < start || code > end) continue;
      const delta = buf.readInt16BE(deltaBase + s * 2);
      const rangeOffset = buf.readUInt16BE(rangeBase + s * 2);
      if (rangeOffset === 0) return (code + delta) & 0xffff;
      const glyph = buf.readUInt16BE(rangeBase + s * 2 + rangeOffset + (code - start) * 2);
      return glyph === 0 ? 0 : (glyph + delta) & 0xffff;
    }
  }
  return 0;
}

const locaBase = tables.loca.offset;
const glyphOffset = (gid) =>
  indexToLocFormat === 0 ? buf.readUInt16BE(locaBase + gid * 2) * 2 : buf.readUInt32BE(locaBase + gid * 4);

function parseGlyph(gid) {
  const start = tables.glyf.offset + glyphOffset(gid);
  const next = tables.glyf.offset + glyphOffset(gid + 1);
  if (next <= start) return [];
  const header = {
    xMin: buf.readInt16BE(start + 2),
    yMin: buf.readInt16BE(start + 4),
    xMax: buf.readInt16BE(start + 6),
    yMax: buf.readInt16BE(start + 8),
  };
  let p = start + 10;
  const numberOfContours = buf.readInt16BE(start);
  if (numberOfContours < 0) throw new Error("compound glyph not supported");
  const endPts = [];
  for (let i = 0; i < numberOfContours; i++) {
    endPts.push(buf.readUInt16BE(p));
    p += 2;
  }
  const numPoints = numberOfContours === 0 ? 0 : endPts[endPts.length - 1] + 1;
  p += 2 + buf.readUInt16BE(p); // instructionLength + instructions

  const flags = [];
  while (flags.length < numPoints) {
    const f = buf.readUInt8(p++);
    flags.push(f);
    if (f & 8) {
      const repeat = buf.readUInt8(p++);
      for (let r = 0; r < repeat; r++) flags.push(f);
    }
  }

  // All x deltas first, then all y deltas (NOT interleaved).
  let x = 0;
  const xs = flags.map((f) => {
    if (f & 2) {
      x += f & 16 ? buf.readUInt8(p) : -buf.readUInt8(p);
      p += 1;
    } else if (!(f & 16)) {
      x += buf.readInt16BE(p);
      p += 2;
    }
    return x;
  });
  let y = 0;
  const ys = flags.map((f) => {
    if (f & 4) {
      y += f & 32 ? buf.readUInt8(p) : -buf.readUInt8(p);
      p += 1;
    } else if (!(f & 32)) {
      y += buf.readInt16BE(p);
      p += 2;
    }
    return y;
  });

  const contours = [];
  let pt = 0;
  for (const end of endPts) {
    const contour = [];
    for (; pt <= end; pt++) contour.push({ x: xs[pt], y: ys[pt], on: (flags[pt] & 1) === 1 });
    contours.push(contour);
  }
  return { contours, header };
}

function contourToPath(contour, map) {
  let startIdx = contour.findIndex((pt) => pt.on);
  let cs = contour;
  if (startIdx < 0) {
    cs = [
      { x: (contour[0].x + contour[1].x) / 2, y: (contour[0].y + contour[1].y) / 2, on: true },
      ...contour,
    ];
    startIdx = 0;
  }
  cs = [...cs.slice(startIdx), ...cs.slice(0, startIdx)];
  const P = (pt) => map(pt.x, pt.y).join(" ");
  let d = `M${P(cs[0])}`;
  let i = 1;
  while (i < cs.length) {
    const pt = cs[i];
    if (pt.on) {
      d += `L${P(pt)}`;
      i++;
    } else {
      const next = cs[(i + 1) % cs.length];
      if (next.on) {
        d += `Q${P(pt)} ${P(next)}`;
        i += 2;
      } else {
        d += `Q${P(pt)} ${map((pt.x + next.x) / 2, (pt.y + next.y) / 2).join(" ")}`;
        i += 1;
      }
    }
  }
  return `${d}Z`;
}

const gid = glyphIdFor(CHAR.codePointAt(0));
if (!gid) throw new Error(`glyph not found for "${CHAR}"`);
const { contours, header } = parseGlyph(gid);

let minX = Infinity;
let maxX = -Infinity;
let minY = Infinity;
let maxY = -Infinity;
for (const c of contours)
  for (const pt of c) {
    minX = Math.min(minX, pt.x);
    maxX = Math.max(maxX, pt.x);
    minY = Math.min(minY, pt.y);
    maxY = Math.max(maxY, pt.y);
  }

const round = (v) => Number(v.toFixed(2));
const selfCheck =
  round(minX) === header.xMin && round(minY) === header.yMin && round(maxX) === header.xMax && round(maxY) === header.yMax;
console.error(
  `glyph ${JSON.stringify(CHAR)} gid=${gid} contours=${contours.length} bbox=[${minX},${minY},${maxX},${maxY}] ` +
    `header=[${header.xMin},${header.yMin},${header.xMax},${header.yMax}] self-check=${selfCheck ? "OK" : "MISMATCH"}`,
);
if (!selfCheck) throw new Error("parsed bbox does not match glyf header bbox — coordinate parsing is wrong");

const s = TARGET_H / (maxY - minY);
const cx = (minX + maxX) / 2;
const cy = (minY + maxY) / 2;
const map = (x, y) => [round((x - cx) * s + VIEW / 2), round(-(y - cy) * s + VIEW / 2)];

console.log(contours.map((c) => contourToPath(c, map)).join(" "));
