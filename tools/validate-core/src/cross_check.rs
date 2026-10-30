//! Cross-artifact checks, gate-chain walking, and JS value-display
//! helpers. All check functions mutate the command's shared errors/notes
//! accumulators directly — the oracle uses single arrays per command and
//! its success-note rule (`errors.length === before`) is global, so these
//! functions receive the same vectors the stage code uses.

use serde_json::Value;
use std::collections::HashSet;

/// JS truthiness — used for `gate.approved`-style fields, where a
/// malformed truthy string ("yes") counts as approved exactly like `!!v`
/// does in the oracle.
pub fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().map(|f| f != 0.0).unwrap_or(true),
        Value::String(s) => !s.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

/// JS template-literal stringification for values interpolated into
/// messages: strings go bare, `undefined` for missing members.
pub fn js_value(v: Option<&Value>) -> String {
    match v {
        None => "undefined".to_string(),
        Some(Value::Null) => "null".to_string(),
        Some(Value::Bool(b)) => b.to_string(),
        Some(Value::Number(n)) => match n.as_f64() {
            Some(f) => format!("{}", f),
            None => n.to_string(),
        },
        Some(Value::String(s)) => s.clone(),
        Some(Value::Array(a)) => a
            .iter()
            .map(|item| match item {
                Value::Null => String::new(),
                other => js_value(Some(other)),
            })
            .collect::<Vec<_>>()
            .join(","),
        Some(Value::Object(_)) => "[object Object]".to_string(),
    }
}

/// `v[key]` as a slice without allocating: missing or non-array → empty.
fn array_of<'a>(v: &'a Value, key: &str) -> &'a [Value] {
    v.get(key)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

fn str_of<'a>(v: &'a Value, key: &str) -> Option<&'a str> {
    v.get(key).and_then(Value::as_str)
}

/// ADR §6.1 `checkPlanCross`.
pub fn check_plan_cross(errors: &mut Vec<String>, notes: &mut Vec<String>, plan: &Value) {
    let before = errors.len();
    let rqs = array_of(plan, "research_questions");
    let rq_ids: HashSet<&str> = rqs.iter().filter_map(|q| str_of(q, "id")).collect();
    if rq_ids.len() != rqs.len() {
        errors.push("plan: duplicate research question ids".to_string());
    }
    for s in array_of(plan, "search_strategy") {
        if let Some(qid) = str_of(s, "question_id") {
            if !rq_ids.contains(qid) {
                errors.push(format!(
                    "plan: search_strategy references unknown question {}",
                    qid
                ));
            }
        }
    }
    let covered: HashSet<&str> = array_of(plan, "search_strategy")
        .iter()
        .filter_map(|s| str_of(s, "question_id"))
        .collect();
    for id in &rq_ids {
        if !covered.contains(id) {
            errors.push(format!("plan: question {} has no search strategy", id));
        }
    }
    if errors.len() == before {
        notes.push(format!(
            "plan: {} question(s), all covered by search strategy",
            rq_ids.len()
        ));
    }
}

/// ADR §6.2 `checkEvidenceCross`. (The run_id check lives in the stage
/// function, as in the oracle — not here.)
pub fn check_evidence_cross(
    errors: &mut Vec<String>,
    notes: &mut Vec<String>,
    plan: &Value,
    ev: &Value,
) {
    let before = errors.len();
    let rq_ids: HashSet<&str> = array_of(plan, "research_questions")
        .iter()
        .filter_map(|q| str_of(q, "id"))
        .collect();
    let mut evidence_ids = HashSet::new();
    for r in array_of(ev, "records") {
        let question_str = str_of(r, "question_id");
        let unknown = match question_str {
            Some(qid) => !rq_ids.contains(qid),
            None => true,
        };
        if unknown {
            errors.push(format!(
                "evidence: record {} references unknown question {}",
                js_value(r.get("evidence_id")),
                js_value(r.get("question_id"))
            ));
        }
        if let Some(eid) = str_of(r, "evidence_id") {
            if !evidence_ids.insert(eid) {
                errors.push(format!("evidence: duplicate evidence id {}", eid));
            }
        }
    }
    let gap_qs: HashSet<&str> = array_of(ev, "gaps")
        .iter()
        .filter_map(|g| str_of(g, "question_id"))
        .collect();
    for id in &rq_ids {
        let has_records = array_of(ev, "records")
            .iter()
            .any(|r| str_of(r, "question_id") == Some(*id));
        if !has_records && !gap_qs.contains(id) {
            errors.push(format!(
                "evidence: question {} has neither evidence nor a recorded gap",
                id
            ));
        }
    }
    if errors.len() == before {
        notes.push(format!(
            "evidence: {} record(s), {} gap(s), every question accounted for",
            array_of(ev, "records").len(),
            array_of(ev, "gaps").len()
        ));
    }
}

/// ADR §6.3 `checkVerificationCross`. `plan` is the already-loaded plan
/// (may be absent); the oracle re-reads it mid-check only in states that
/// also suppress this whole check.
pub fn check_verification_cross(
    errors: &mut Vec<String>,
    notes: &mut Vec<String>,
    plan: Option<&Value>,
    ev: &Value,
    ver: &Value,
) {
    let before = errors.len();
    let evidence_ids: HashSet<&str> = array_of(ev, "records")
        .iter()
        .filter_map(|r| str_of(r, "evidence_id"))
        .collect();
    let mut result_ids = HashSet::new();
    for r in array_of(ver, "results") {
        if let Some(vid) = str_of(r, "result_id") {
            if !result_ids.insert(vid) {
                errors.push(format!("verification: duplicate result id {}", vid));
            }
        }
        for eid in array_of(r, "evidence_ids") {
            if let Some(eid_str) = eid.as_str() {
                if !evidence_ids.contains(eid_str) {
                    errors.push(format!(
                        "verification: result {} references unknown evidence {}",
                        js_value(r.get("result_id")),
                        eid_str
                    ));
                }
            }
        }
    }
    // Contradicting evidence that no result adjudicates.
    for r in array_of(ev, "records") {
        if str_of(r, "support_relation") == Some("contradicts") {
            let eid = str_of(r, "evidence_id").unwrap_or("");
            let adjudicated = array_of(ver, "results").iter().any(|v| {
                array_of(v, "evidence_ids")
                    .iter()
                    .any(|e| e.as_str() == Some(eid))
            });
            if !adjudicated {
                errors.push(format!(
                    "verification: contradicting evidence {} is never adjudicated in a result",
                    eid
                ));
            }
        }
    }
    // Question coverage through cited evidence.
    let covered: HashSet<&str> = array_of(ver, "results")
        .iter()
        .flat_map(|v| array_of(v, "evidence_ids").iter())
        .filter_map(|eid| {
            let eid_str = eid.as_str()?;
            array_of(ev, "records")
                .iter()
                .find(|r| str_of(r, "evidence_id") == Some(eid_str))
                .and_then(|r| str_of(r, "question_id"))
        })
        .collect();
    if let Some(plan) = plan {
        for q in array_of(plan, "research_questions") {
            if let Some(qid) = str_of(q, "id") {
                if !covered.contains(qid) {
                    errors.push(format!(
                        "verification: question {} has no verification result",
                        qid
                    ));
                }
            }
        }
    }
    if errors.len() == before {
        notes.push(format!(
            "verification: {} result(s), contradictions adjudicated, every question verified",
            array_of(ver, "results").len()
        ));
    }
}

/// ADR §6.4 `checkReportCross`. (The run_id check lives in the stage
/// function — report is compared against plan only.)
pub fn check_report_cross(
    errors: &mut Vec<String>,
    notes: &mut Vec<String>,
    plan: &Value,
    ev: &Value,
    ver: &Value,
    rep: &Value,
) {
    let before = errors.len();
    let evidence_ids: HashSet<&str> = array_of(ev, "records")
        .iter()
        .filter_map(|r| str_of(r, "evidence_id"))
        .collect();
    let cite = |ids: &[Value], at: &str, out: &mut Vec<String>| {
        for eid in ids {
            if let Some(eid_str) = eid.as_str() {
                if !evidence_ids.contains(eid_str) {
                    out.push(format!("report: {} cites unknown evidence {}", at, eid_str));
                }
            }
        }
    };

    for (i, f) in array_of(rep, "key_findings").iter().enumerate() {
        cite(
            array_of(f, "evidence_ids"),
            &format!("key_findings[{}]", i),
            errors,
        );
    }
    for (i, c) in array_of(rep, "conflicts").iter().enumerate() {
        cite(
            array_of(c, "evidence_ids"),
            &format!("conflicts[{}]", i),
            errors,
        );
    }
    for (i, s) in array_of(rep, "sources").iter().enumerate() {
        cite(
            array_of(s, "evidence_ids"),
            &format!("sources[{}]", i),
            errors,
        );
    }

    let analyzed: HashSet<&str> = array_of(rep, "detailed_analysis")
        .iter()
        .filter_map(|d| str_of(d, "question_id"))
        .collect();
    for q in array_of(plan, "research_questions") {
        if let Some(qid) = str_of(q, "id") {
            if !analyzed.contains(qid) {
                errors.push(format!(
                    "report: question {} has no detailed_analysis section",
                    qid
                ));
            }
        }
    }
    let verified: HashSet<&str> = array_of(ver, "results")
        .iter()
        .filter(|r| {
            str_of(r, "status") == Some("verified")
                || str_of(r, "status") == Some("partially_supported")
        })
        .filter_map(|r| str_of(r, "result_id"))
        .collect();
    if errors.len() == before {
        notes.push(format!(
            "report: {} finding(s), {} verified/partial result(s) behind the chain, all questions analyzed",
            array_of(rep, "key_findings").len(),
            verified.len()
        ));
    }
}

/// Docs needed by `checkGatesInOrder`. The oracle loads all four gates
/// (in plan → evidence → verification → report order) whenever it runs,
/// so the caller builds this with the same loads for identical parse-error
/// side effects.
pub struct GateDocs {
    pub plan: Option<Value>,
    pub evidence: Option<Value>,
    pub verification: Option<Value>,
    pub report: Option<Value>,
}

impl GateDocs {
    fn stage(&self, stage: &str) -> Option<&Value> {
        match stage {
            "plan" => self.plan.as_ref(),
            "evidence" => self.evidence.as_ref(),
            "verification" => self.verification.as_ref(),
            _ => self.report.as_ref(),
        }
    }
}

/// ADR §6.5 `checkGatesInOrder`.
pub fn check_gates_in_order(
    errors: &mut Vec<String>,
    notes: &mut Vec<String>,
    needed: &[&str],
    docs: &GateDocs,
) {
    let mut prev_approved = true;
    let mut pending: Vec<&str> = Vec::new();
    for &stage in needed {
        let gate = docs.stage(stage).and_then(|doc| doc.get("gate"));
        let Some(gate) = gate else {
            errors.push(format!("{}: artifact missing, chain broken", stage));
            break;
        };
        if !prev_approved {
            errors.push(format!(
                "{}: previous stage's gate was never approved — this stage may not start",
                stage
            ));
            break;
        }
        let approved = gate.get("approved").map(truthy).unwrap_or(false);
        prev_approved = approved;
        if !approved {
            pending.push(stage);
        }
    }
    if errors.is_empty() && pending.is_empty() {
        notes.push(format!(
            "gate chain: {} approved in order",
            needed.join(" → ")
        ));
    }
    if errors.is_empty() && !pending.is_empty() {
        notes.push(format!(
            "gate chain: valid through {}; awaiting human gate approval on: {}",
            needed.join(" → "),
            pending.join(", ")
        ));
    }
}
