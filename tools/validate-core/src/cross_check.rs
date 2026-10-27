//! Cross-artifact checks and gate ordering.

use crate::schemas::SchemaCache;
use serde_json::Value;
use std::collections::HashSet;

pub struct CrossChecker {
    errors: Vec<String>,
    notes: Vec<String>,
}

impl CrossChecker {
    pub fn new(_cache: SchemaCache) -> Self {
        Self {
            errors: Vec::new(),
            notes: Vec::new(),
        }
    }

    pub fn errors(&self) -> &[String] {
        &self.errors
    }
    pub fn notes(&self) -> &[String] {
        &self.notes
    }
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }
    pub fn take(self) -> (Vec<String>, Vec<String>) {
        (self.errors, self.notes)
    }

    fn add_error(&mut self, msg: &str) {
        self.errors.push(msg.to_string());
    }
    fn add_note(&mut self, msg: &str) {
        self.notes.push(msg.to_string());
    }

    fn note_if_no_errors(&mut self, msg: &str) {
        if self.errors.is_empty() {
            self.add_note(msg);
        }
    }

    /// Check that two artifacts have matching run_id
    fn check_run_id_match(&mut self, a: &Value, b: &Value, a_name: &str, b_name: &str) {
        let a_id = a["run_id"].as_str().unwrap_or("");
        let b_id = b["run_id"].as_str().unwrap_or("");
        if a_id != b_id && !a_id.is_empty() && !b_id.is_empty() {
            self.add_error(&format!(
                "{}: run_id mismatch with {} (\"{}\" vs \"{}\")",
                a_name, b_name, a_id, b_id
            ));
        }
    }

    pub fn check_plan_cross(&mut self, plan: &Value) {
        let rq_ids: HashSet<_> = plan["research_questions"]
            .as_array()
            .map(|arr| arr.iter().filter_map(|q| q["id"].as_str()).collect())
            .unwrap_or_default();
        if rq_ids.len()
            != plan["research_questions"]
                .as_array()
                .map(|a| a.len())
                .unwrap_or(0)
        {
            self.add_error("plan: duplicate research question ids");
        }
        for s in plan["search_strategy"].as_array().unwrap_or(&vec![]) {
            if let Some(qid) = s["question_id"].as_str() {
                if !rq_ids.contains(qid) {
                    self.add_error(&format!(
                        "plan: search_strategy references unknown question {}",
                        qid
                    ));
                }
            }
        }
        let covered: HashSet<_> = plan["search_strategy"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|s| s["question_id"].as_str())
                    .collect()
            })
            .unwrap_or_default();
        for id in &rq_ids {
            if !covered.contains(id) {
                self.add_error(&format!("plan: question {} has no search strategy", id));
            }
        }
        self.note_if_no_errors(&format!(
            "plan: {} question(s), all covered by search strategy",
            rq_ids.len()
        ));
    }

    pub fn check_evidence_cross(&mut self, plan: &Value, ev: &Value) {
        // Check run_id match
        self.check_run_id_match(ev, plan, "evidence", "plan");

        let rq_ids: HashSet<_> = plan["research_questions"]
            .as_array()
            .map(|arr| arr.iter().filter_map(|q| q["id"].as_str()).collect())
            .unwrap_or_default();
        let mut ev_ids = HashSet::new();
        for r in ev["records"].as_array().unwrap_or(&vec![]) {
            if let Some(qid) = r["question_id"].as_str() {
                if !rq_ids.contains(qid) {
                    self.add_error(&format!(
                        "evidence: record {} references unknown question {}",
                        r["evidence_id"].as_str().unwrap_or("?"),
                        qid
                    ));
                }
            }
            if let Some(eid) = r["evidence_id"].as_str() {
                if !ev_ids.insert(eid) {
                    self.add_error(&format!("evidence: duplicate evidence id {}", eid));
                }
            }
        }
        let gap_qs: HashSet<_> = ev["gaps"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|g| g["question_id"].as_str())
                    .collect()
            })
            .unwrap_or_default();
        for id in &rq_ids {
            let has_evidence = ev["records"]
                .as_array()
                .map(|arr| arr.iter().any(|r| r["question_id"].as_str() == Some(*id)))
                .unwrap_or(false);
            if !has_evidence && !gap_qs.contains(id) {
                self.add_error(&format!(
                    "evidence: question {} has neither evidence nor a recorded gap",
                    id
                ));
            }
        }
        self.note_if_no_errors(&format!(
            "evidence: {} record(s), {} gap(s), every question accounted for",
            ev["records"].as_array().map(|a| a.len()).unwrap_or(0),
            ev["gaps"].as_array().map(|a| a.len()).unwrap_or(0)
        ));
    }

    pub fn check_verification_cross(&mut self, plan: &Value, ev: &Value, ver: &Value) {
        // Check run_id match
        self.check_run_id_match(ver, plan, "verification", "plan");
        self.check_run_id_match(ver, ev, "verification", "evidence");

        let ev_ids: HashSet<_> = ev["records"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|r| r["evidence_id"].as_str())
                    .collect()
            })
            .unwrap_or_default();
        let mut v_ids = HashSet::new();
        for r in ver["results"].as_array().unwrap_or(&vec![]) {
            if let Some(vid) = r["result_id"].as_str() {
                if !v_ids.insert(vid) {
                    self.add_error(&format!("verification: duplicate result id {}", vid));
                }
            }
            for eid in r["evidence_ids"].as_array().unwrap_or(&vec![]) {
                if let Some(eid_str) = eid.as_str() {
                    if !ev_ids.contains(eid_str) {
                        self.add_error(&format!(
                            "verification: result {} references unknown evidence {}",
                            r["result_id"].as_str().unwrap_or("?"),
                            eid_str
                        ));
                    }
                }
            }
        }
        // Unadjudicated contradicting evidence
        for r in ev["records"].as_array().unwrap_or(&vec![]) {
            if r["support_relation"].as_str() == Some("contradicts") {
                let eid = r["evidence_id"].as_str().unwrap_or("");
                let adjudicated = ver["results"]
                    .as_array()
                    .map(|arr| {
                        arr.iter().any(|v| {
                            v["evidence_ids"]
                                .as_array()
                                .map(|eids| eids.iter().any(|e| e.as_str() == Some(eid)))
                                .unwrap_or(false)
                        })
                    })
                    .unwrap_or(false);
                if !adjudicated {
                    self.add_error(&format!(
                        "verification: contradicting evidence {} is never adjudicated in a result",
                        eid
                    ));
                }
            }
        }
        // Every plan question covered
        let mut covered_qs = HashSet::new();
        let empty_vec = vec![];
        let records = ev["records"].as_array().unwrap_or(&empty_vec);
        for v in ver["results"].as_array().unwrap_or(&vec![]) {
            for eid in v["evidence_ids"].as_array().unwrap_or(&vec![]) {
                if let Some(eid_str) = eid.as_str() {
                    if let Some(qid) = records
                        .iter()
                        .find(|r| r["evidence_id"].as_str() == Some(eid_str))
                        .and_then(|r| r["question_id"].as_str())
                    {
                        covered_qs.insert(qid);
                    }
                }
            }
        }
        for q in plan["research_questions"].as_array().unwrap_or(&vec![]) {
            if let Some(qid) = q["id"].as_str() {
                if !covered_qs.contains(qid) {
                    self.add_error(&format!(
                        "verification: question {} has no verification result",
                        qid
                    ));
                }
            }
        }
        self.note_if_no_errors(&format!(
            "verification: {} result(s), contradictions adjudicated, every question verified",
            ver["results"].as_array().map(|a| a.len()).unwrap_or(0)
        ));
    }

    pub fn check_report_cross(&mut self, plan: &Value, ev: &Value, ver: &Value, rep: &Value) {
        // Check run_id match
        self.check_run_id_match(rep, plan, "report", "plan");
        self.check_run_id_match(rep, ev, "report", "evidence");
        self.check_run_id_match(rep, ver, "report", "verification");

        let ev_ids: HashSet<_> = ev["records"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|r| r["evidence_id"].as_str())
                    .collect()
            })
            .unwrap_or_default();
        let rq_ids: HashSet<_> = plan["research_questions"]
            .as_array()
            .map(|arr| arr.iter().filter_map(|q| q["id"].as_str()).collect())
            .unwrap_or_default();

        for (i, f) in rep["key_findings"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .enumerate()
        {
            for eid in f["evidence_ids"].as_array().unwrap_or(&vec![]) {
                if let Some(eid_str) = eid.as_str() {
                    if !ev_ids.contains(eid_str) {
                        self.add_error(&format!(
                            "report: key_findings[{}] cites unknown evidence {}",
                            i, eid_str
                        ));
                    }
                }
            }
        }
        for (i, c) in rep["conflicts"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .enumerate()
        {
            for eid in c["evidence_ids"].as_array().unwrap_or(&vec![]) {
                if let Some(eid_str) = eid.as_str() {
                    if !ev_ids.contains(eid_str) {
                        self.add_error(&format!(
                            "report: conflicts[{}] cites unknown evidence {}",
                            i, eid_str
                        ));
                    }
                }
            }
        }
        for (i, s) in rep["sources"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .enumerate()
        {
            for eid in s["evidence_ids"].as_array().unwrap_or(&vec![]) {
                if let Some(eid_str) = eid.as_str() {
                    if !ev_ids.contains(eid_str) {
                        self.add_error(&format!(
                            "report: sources[{}] cites unknown evidence {}",
                            i, eid_str
                        ));
                    }
                }
            }
        }
        let analyzed: HashSet<_> = rep["detailed_analysis"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|d| d["question_id"].as_str())
                    .collect()
            })
            .unwrap_or_default();
        for id in &rq_ids {
            if !analyzed.contains(id) {
                self.add_error(&format!(
                    "report: question {} has no detailed_analysis section",
                    id
                ));
            }
        }
        let verified: HashSet<_> = ver["results"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter(|r| {
                        r["status"].as_str() == Some("verified")
                            || r["status"].as_str() == Some("partially_supported")
                    })
                    .filter_map(|r| r["result_id"].as_str())
                    .collect()
            })
            .unwrap_or_default();
        self.note_if_no_errors(&format!("report: {} finding(s), {} verified/partial result(s) behind the chain, all questions analyzed", 
            rep["key_findings"].as_array().map(|a| a.len()).unwrap_or(0),
            verified.len()));
    }

    pub fn check_gates_in_order<F>(&mut self, needed: &[&str], mut loaders: F)
    where
        F: FnMut(&str) -> Option<Value>,
    {
        let mut prev_approved = true;
        let mut pending = Vec::new();
        for stage in needed {
            let doc = loaders(stage);
            if doc.is_none() {
                self.add_error(&format!("{}: artifact missing, chain broken", stage));
                break;
            }
            let gate = doc.as_ref().map(|v| v["gate"].clone());
            if gate.as_ref().map(|g| g.is_null()).unwrap_or(true) {
                self.add_error(&format!("{}: artifact missing, chain broken", stage));
                break;
            }
            if !prev_approved {
                self.add_error(&format!(
                    "{}: previous stage's gate was never approved — this stage may not start",
                    stage
                ));
                break;
            }
            let approved = gate
                .as_ref()
                .and_then(|g| g["approved"].as_bool())
                .unwrap_or(false);
            prev_approved = approved;
            if !approved {
                pending.push(*stage);
            }
        }
        if self.errors.is_empty() {
            if pending.is_empty() {
                self.add_note(&format!(
                    "gate chain: {} approved in order",
                    needed.join(" → ")
                ));
            } else {
                self.add_note(&format!(
                    "gate chain: valid through {}; awaiting human gate approval on: {}",
                    needed.join(" → "),
                    pending.join(", ")
                ));
            }
        }
    }
}
