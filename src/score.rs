//! Compare findings with an answer key: precision, recall and F1, overall and per rule.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::rules::{Finding, Severity};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct Expected {
    pub rule: String,
    pub url: String,
    pub why: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnswerKey {
    pub seed: u64,
    pub origin: String,
    pub expected: Vec<Expected>,
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct Counts {
    pub true_positives: usize,
    pub false_positives: usize,
    pub false_negatives: usize,
}

impl Counts {
    pub fn precision(&self) -> f64 {
        ratio(self.true_positives, self.true_positives + self.false_positives)
    }
    pub fn recall(&self) -> f64 {
        ratio(self.true_positives, self.true_positives + self.false_negatives)
    }
    pub fn f1(&self) -> f64 {
        let (p, r) = (self.precision(), self.recall());
        if p + r == 0.0 {
            0.0
        } else {
            2.0 * p * r / (p + r)
        }
    }
}

fn ratio(a: usize, b: usize) -> f64 {
    if b == 0 {
        1.0
    } else {
        a as f64 / b as f64
    }
}

#[derive(Debug, Serialize)]
pub struct Score {
    pub overall: Counts,
    pub per_rule: BTreeMap<String, Counts>,
    pub missed: Vec<(String, String)>,
    pub extra: Vec<(String, String)>,
}

/// Notices are information, not defects: they are left out on both sides.
pub fn score(findings: &[Finding], key: &AnswerKey) -> Score {
    let got: BTreeSet<(String, String)> =
        findings.iter().filter(|f| f.severity != Severity::Notice).map(|f| (f.rule.clone(), f.url.clone())).collect();
    let want: BTreeSet<(String, String)> = key.expected.iter().map(|e| (e.rule.clone(), e.url.clone())).collect();
    let mut per_rule: BTreeMap<String, Counts> = BTreeMap::new();
    let mut overall = Counts::default();
    for item in got.union(&want) {
        let c = per_rule.entry(item.0.clone()).or_default();
        match (got.contains(item), want.contains(item)) {
            (true, true) => {
                c.true_positives += 1;
                overall.true_positives += 1;
            }
            (true, false) => {
                c.false_positives += 1;
                overall.false_positives += 1;
            }
            (false, true) => {
                c.false_negatives += 1;
                overall.false_negatives += 1;
            }
            _ => {}
        }
    }
    Score {
        overall,
        per_rule,
        missed: want.difference(&got).cloned().collect(),
        extra: got.difference(&want).cloned().collect(),
    }
}
