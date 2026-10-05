#!/usr/bin/env node
// Writes preview/assets/avatars/<slug>.svg: flat, geometric demo avatars (no faces, no third-party art).
// Dependency-free (Node stdlib only) and deterministic: the same names always give the same files.
// Usage: node scripts/gen-demo-avatars.mjs
import { writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

const OUT = fileURLToPath(new URL("../preview/assets/avatars/", import.meta.url));
const NAMES = ["Avery Lin", "Casey Park", "Robin Hayes", "Sarah Chen", "Marcus Wright", "Lena Park", "Jordan Reyes", "Priya Nair"];
// [field, shape A, shape B]: mid-tone fields so a disc reads on a white and on a near-black page.
const PALETTES = [
  ["#a9bfa6", "#35564a", "#e9b44c"], ["#e0b8a6", "#8a3b2c", "#2f3a4a"], ["#a5bbd4", "#243b66", "#e8896b"],
  ["#d9c79e", "#3e4a2e", "#b5503c"], ["#bcaed3", "#47306b", "#f0c15d"], ["#a4c6c4", "#1f5a5a", "#e07a5f"],
  ["#dbb0b9", "#7b2d45", "#2f4858"], ["#bdbab2", "#2f3a4a", "#d98a3d"],
];
const CREAM = "#f6f1e7";
const c = (x, y, r, f) => `<circle cx="${x}" cy="${y}" r="${r}" fill="${f}"/>`;
const r = (x, y, w, h, f) => `<rect x="${x}" y="${y}" width="${w}" height="${h}" fill="${f}"/>`;
const p = (d, f) => `<path d="${d}" fill="${f}"/>`;
// Eight compositions (a, b = palette shapes, n = cream, j = small per-name offset 0..6).
const TEMPLATES = [
  (a, b, n, j) => c(40, 32 + j / 2, 19, a) + r(0, 54, 80, 26, b), // sun over a horizon
  (a, b, n, j) => p("M20 80V44a20 20 0 0 1 40 0V80z", a) + c(60 - j, 20, 8, b), // arch + dot
  (a, b, n, j) => p(`M${37 + j} 12L70 64H10z`, a) + c(40, 54, 7, n) + r(0, 68, 80, 12, b), // triangle
  (a, b, n, j) => p("M0 0H58A58 58 0 0 1 0 58z", a) + c(58, 58, 14 + j / 2, b), // quarter disc
  (a, b, n, j) => r(0, 22 + j, 80, 22, a) + c(40, 33 + j, 11, n) + r(0, 58, 80, 22, b), // bands + dot
  (a, b, n, j) => p("M0 80L80 0V80z", a) + c(27, 27 + j, 13, b) + r(10, 54, 18, 18, n), // diagonal
  (a, b, n, j) => p("M10 54a30 30 0 0 1 60 0z", a) + r(10, 54, 60, 8, b) + c(40, 70 + j / 2, 5, n), // dome + bar
  (a, b, n, j) => c(40, 40, 28, a) + c(40, 40, 16, b) + c(40, 40, 6 + j / 2, n), // target
];
// FNV-1a: the name picks its composition, palette and offset; a clash with an earlier name steps to the next free one.
const hash = (s) => [...s].reduce((h, ch) => Math.imul(h ^ ch.charCodeAt(0), 16777619) >>> 0, 2166136261);
const taken = { t: new Set(), p: new Set() };
const pick = (kind, start, size) => { let i = start % size; while (taken[kind].has(i)) i = (i + 1) % size; taken[kind].add(i); return i; };

for (const name of NAMES) {
  const h = hash(name);
  const [bg, a, b] = PALETTES[pick("p", h >>> 8, PALETTES.length)];
  const shapes = TEMPLATES[pick("t", h, TEMPLATES.length)](a, b, CREAM, (h >>> 20) % 7);
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 80 80">${r(0, 0, 80, 80, bg)}${shapes}</svg>\n`;
  writeFileSync(OUT + name.toLowerCase().replace(/\s+/g, "-") + ".svg", svg);
}
