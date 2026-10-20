#!/usr/bin/env node
// validate.mjs — deterministic boundary for the deep-research-agent workspace.
// Zero dependencies (Node >= 18). Validates stage artifacts against the
// project-owned schemas in _shared/schemas/ and enforces cross-artifact
// integrity, gate ordering, and run completion.
//
// Usage:
//   node tools/validate.mjs status          # scan stages/*/output/, report run state
//   node tools/validate.mjs plan             # validate stage 01 artifact (+ upstream gates)
//   node tools/validate.mjs evidence         # validate stage 02 (+ plan cross-refs)
//   node tools/validate.mjs verification     # validate stage 03 (+ evidence cross-refs)
//   node tools/validate.mjs report           # validate stage 04 (+ all cross-refs)
//   node tools/validate.mjs run              # whole chain + completion verdict
//   node tools/validate.mjs file <path> <schema-name>   # ad-hoc: validate any JSON against a schema
//
// Deterministic checks confirm a citation exists and resolves to a recorded
// evidence object. They cannot establish that the source truly supports the
// claim — that stays a human + evidence-auditor judgment.

import { readFileSync, existsSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const ROOT = dirname(dirname(fileURLToPath(import.meta.url)));
const SCHEMAS = join(ROOT, "_shared", "schemas");
const OUT = {
  plan: join(ROOT, "stages", "01_plan", "output", "plan.json"),
  evidence: join(ROOT, "stages", "02_research", "output", "evidence.json"),
  verification: join(ROOT, "stages", "03_verify", "output", "verification.json"),
  report: join(ROOT, "stages", "04_report", "output", "report.json"),
};

const errors = [];
const notes = [];
function fail(msg) { errors.push(msg); }
function ok(msg) { notes.push(msg); }

// ---------- tiny JSON-Schema subset validator ----------
const schemaCache = {};
function loadSchema(name) {
  if (schemaCache[name]) return schemaCache[name];
  const candidates = [name, `${name}.json`, `${name}.schema.json`].filter((n) => n.endsWith(".json"));
  let path;
  for (const c of candidates) {
    const p = join(SCHEMAS, c);
    if (existsSync(p)) { path = p; break; }
  }
  if (!path) throw new Error(`schema not found: ${candidates.join(", ")}`);
  schemaCache[name] = { doc: JSON.parse(readFileSync(path, "utf8")), dir: SCHEMAS };
  return schemaCache[name];
}
function resolveRef(ref) {
  const [file, pointer] = ref.split("#");
  const { doc } = loadSchema(file);
  if (!pointer) return doc;
  let node = doc;
  for (const seg of pointer.split("/").filter(Boolean)) {
    node = node[decodeURIComponent(seg)];
    if (node === undefined) throw new Error(`bad $ref pointer: ${ref}`);
  }
  return node;
}
function validate(value, schema, at) {
  if (schema.$ref) return validate(value, resolveRef(schema.$ref), at);
  if (schema.enum && !schema.enum.includes(value))
    fail(`${at}: expected one of [${schema.enum.join(", ")}], got ${JSON.stringify(value)}`);
  const t = schema.type;
  if (t) {
    const actual = Array.isArray(value) ? "array" : typeof value === "number" && Number.isInteger(value) && t === "integer" ? "integer" : typeof value;
    if (t === "integer" ? !Number.isInteger(value) : actual !== t)
      if (!(t === "number" && typeof value === "number")) fail(`${at}: expected ${t}, got ${actual}`);
  }
  if (typeof value === "string") {
    if (schema.minLength !== undefined && value.length < schema.minLength) fail(`${at}: shorter than minLength ${schema.minLength}`);
    if (schema.pattern && !new RegExp(schema.pattern).test(value)) fail(`${at}: does not match pattern ${schema.pattern} (got ${JSON.stringify(value)})`);
  }
  if (Array.isArray(value)) {
    if (schema.minItems !== undefined && value.length < schema.minItems) fail(`${at}: fewer than ${schema.minItems} item(s)`);
    if (schema.items) value.forEach((v, i) => validate(v, schema.items, `${at}[${i}]`));
  }
  if (value && typeof value === "object" && !Array.isArray(value)) {
    for (const key of schema.required || []) if (!(key in value)) fail(`${at}: missing required field "${key}"`);
    if (schema.properties) {
      for (const [key, sub] of Object.entries(schema.properties)) if (key in value) validate(value[key], sub, `${at}.${key}`);
    }
    if (schema.additionalProperties === false && schema.properties) {
      for (const key of Object.keys(value)) if (!(key in schema.properties)) fail(`${at}: unknown field "${key}"`);
    }
  }
  return errors.length === 0;
}

// ---------- loaders ----------
function loadJson(path, label) {
  if (!existsSync(path)) return null;
  try { return JSON.parse(readFileSync(path, "utf8")); }
  catch (e) { fail(`${label}: not valid JSON (${e.message})`); return null; }
}

// ---------- cross-artifact integrity ----------
function planOf() { return loadJson(OUT.plan, "plan"); }
function evidenceOf() { return loadJson(OUT.evidence, "evidence"); }

function checkPlanCross(plan) {
  const before = errors.length;
  const rq = new Set(plan.research_questions.map((q) => q.id));
  if (rq.size !== plan.research_questions.length) fail("plan: duplicate research question ids");
  for (const s of plan.search_strategy) if (!rq.has(s.question_id)) fail(`plan: search_strategy references unknown question ${s.question_id}`);
  const covered = new Set(plan.search_strategy.map((s) => s.question_id));
  for (const id of rq) if (!covered.has(id)) fail(`plan: question ${id} has no search strategy`);
  if (errors.length === before) ok(`plan: ${rq.size} question(s), all covered by search strategy`);
}
function checkEvidenceCross(plan, ev) {
  const before = errors.length;
  const rq = new Set(plan.research_questions.map((q) => q.id));
  const evIds = new Set();
  for (const r of ev.records) {
    if (!rq.has(r.question_id)) fail(`evidence: record ${r.evidence_id} references unknown question ${r.question_id}`);
    if (evIds.has(r.evidence_id)) fail(`evidence: duplicate evidence id ${r.evidence_id}`);
    evIds.add(r.evidence_id);
  }
  const gapQs = new Set(ev.gaps.map((g) => g.question_id));
  for (const id of rq) {
    const has = ev.records.some((r) => r.question_id === id);
    if (!has && !gapQs.has(id)) fail(`evidence: question ${id} has neither evidence nor a recorded gap`);
  }
  if (errors.length === before) ok(`evidence: ${ev.records.length} record(s), ${ev.gaps.length} gap(s), every question accounted for`);
}
function checkVerificationCross(ev, ver) {
  const before = errors.length;
  const evIds = new Set(ev.records.map((r) => r.evidence_id));
  const vIds = new Set();
  for (const r of ver.results) {
    if (vIds.has(r.result_id)) fail(`verification: duplicate result id ${r.result_id}`);
    vIds.add(r.result_id);
    for (const id of r.evidence_ids) if (!evIds.has(id)) fail(`verification: result ${r.result_id} references unknown evidence ${id}`);
  }
  const unadjudicated = ev.records.filter((r) => r.support_relation === "contradicts" && !ver.results.some((v) => v.evidence_ids.includes(r.evidence_id)));
  for (const r of unadjudicated) fail(`verification: contradicting evidence ${r.evidence_id} is never adjudicated in a result`);
  const coveredQs = new Set(ver.results.flatMap((r) => r.evidence_ids.map((id) => ev.records.find((x) => x.evidence_id === id)?.question_id).filter(Boolean)));
  const plan = planOf();
  if (plan) for (const q of plan.research_questions) if (!coveredQs.has(q.id)) fail(`verification: question ${q.id} has no verification result`);
  if (errors.length === before) ok(`verification: ${ver.results.length} result(s), contradictions adjudicated, every question verified`);
}
function checkReportCross(plan, ev, ver, rep) {
  const before = errors.length;
  const evIds = new Set(ev.records.map((r) => r.evidence_id));
  const rq = new Set(plan.research_questions.map((q) => q.id));
  const cite = (ids, at) => { for (const id of ids) if (!evIds.has(id)) fail(`report: ${at} cites unknown evidence ${id}`); };
  rep.key_findings.forEach((f, i) => cite(f.evidence_ids, `key_findings[${i}]`));
  (rep.conflicts || []).forEach((c, i) => cite(c.evidence_ids || [], `conflicts[${i}]`));
  rep.sources.forEach((s, i) => cite(s.evidence_ids, `sources[${i}]`));
  const analyzed = new Set(rep.detailed_analysis.map((d) => d.question_id));
  for (const id of rq) if (!analyzed.has(id)) fail(`report: question ${id} has no detailed_analysis section`);
  const verified = new Set(ver.results.filter((r) => r.status === "verified" || r.status === "partially_supported").map((r) => r.result_id));
  if (errors.length === before) ok(`report: ${rep.key_findings.length} finding(s), ${verified.size} verified/partial result(s) behind the chain, all questions analyzed`);
}
function checkGatesInOrder(needed) {
  const gates = { plan: planOf()?.gate, evidence: evidenceOf()?.gate, verification: loadJson(OUT.verification, "verification")?.gate, report: loadJson(OUT.report, "report")?.gate };
  let prevApproved = true;
  for (const stage of needed) {
    const g = gates[stage];
    if (!g) { fail(`${stage}: artifact missing, chain broken`); break; }
    if (!prevApproved) fail(`${stage}: previous stage's gate was never approved — this stage may not start`);
    prevApproved = !!g.approved;
  }
  const last = needed[needed.length - 1];
  if (errors.length === 0) ok(`gate chain: ${needed.join(" → ")} approved in order`);
}

// ---------- stage validators ----------
function stagePlan() {
  const plan = loadJson(OUT.plan, "plan");
  if (!plan) { fail("plan: no artifact — stage 01 not run"); return null; }
  validate(plan, loadSchema("research-plan.schema.json").doc, "plan");
  if (errors.length === 0) checkPlanCross(plan);
  return plan;
}
function stageEvidence() {
  const ev = loadJson(OUT.evidence, "evidence");
  if (!ev) { fail("evidence: no artifact — stage 02 not run"); return null; }
  validate(ev, loadSchema("evidence-bundle.schema.json").doc, "evidence");
  const plan = planOf();
  if (plan && ev.run_id !== plan.run_id) fail(`evidence: run_id "${ev.run_id}" does not match plan's "${plan.run_id}"`);
  if (errors.length === 0 && plan) checkEvidenceCross(plan, ev);
  return ev;
}
function stageVerification() {
  const ver = loadJson(OUT.verification, "verification");
  if (!ver) { fail("verification: no artifact — stage 03 not run"); return null; }
  validate(ver, loadSchema("verification-report.schema.json").doc, "verification");
  const ev = evidenceOf();
  if (ev && ver.run_id !== ev.run_id) fail(`verification: run_id mismatch`);
  if (errors.length === 0 && ev) checkVerificationCross(ev, ver);
  return ver;
}
function stageReport() {
  const rep = loadJson(OUT.report, "report");
  if (!rep) { fail("report: no artifact — stage 04 not run"); return null; }
  validate(rep, loadSchema("research-report.schema.json").doc, "report");
  const plan = planOf(), ev = evidenceOf(), ver = loadJson(OUT.verification, "verification");
  if (plan && rep.run_id !== plan.run_id) fail(`report: run_id mismatch`);
  if (errors.length === 0 && plan && ev && ver) checkReportCross(plan, ev, ver, rep);
  if (existsSync(join(ROOT, "stages", "04_report", "output", "report.md"))) ok("report: report.md rendering exists");
  else fail("report: report.md rendering missing");
  return rep;
}

// ---------- commands ----------
const cmd = process.argv[2] ?? "status";

if (cmd === "file") {
  const [path, schemaName] = process.argv.slice(3);
  const doc = loadJson(path, path);
  if (doc) { validate(doc, loadSchema(schemaName).doc, path); }
} else if (cmd === "status") {
  const table = [
    ["01 plan", OUT.plan], ["02 evidence", OUT.evidence],
    ["03 verification", OUT.verification], ["04 report", OUT.report],
  ];
  let produced = 0;
  for (const [label, path] of table) {
    const doc = loadJson(path, label);
    const state = !doc ? "—" : doc.gate?.approved ? "COMPLETE (gate approved)" : "PRODUCED (gate pending)";
    if (doc) produced++;
    console.log(`${label}: ${state}`);
  }
  console.log(produced === 4 ? "Run state: chain complete — archive via _templates/run → runs/" : `Run state: in progress (${produced}/4 artifacts)`);
  process.exit(0);
} else if (cmd === "plan") stagePlan();
else if (cmd === "evidence") { checkGatesInOrder(["plan"]); stageEvidence(); }
else if (cmd === "verification") { checkGatesInOrder(["plan", "evidence"]); stageVerification(); }
else if (cmd === "report") { checkGatesInOrder(["plan", "evidence", "verification"]); stageReport(); }
else if (cmd === "run") {
  const plan = stagePlan(); const ev = stageEvidence(); const ver = stageVerification(); const rep = stageReport();
  checkGatesInOrder(["plan", "evidence", "verification", "report"]);
  if (errors.length === 0 && plan && ev && ver && rep) {
    console.log("RUN COMPLETE — all artifacts validate, all gates approved in order.");
  }
} else {
  console.error(`unknown command: ${cmd}`);
  process.exit(2);
}

for (const n of notes) console.log(`  ok    ${n}`);
if (errors.length) {
  for (const e of errors) console.error(`  FAIL  ${e}`);
  console.error(`${errors.length} check(s) failed.`);
  process.exit(1);
}
console.log("All deterministic checks passed. Schema validity ≠ source truth — the evidence-auditor and human gates still own that.");
