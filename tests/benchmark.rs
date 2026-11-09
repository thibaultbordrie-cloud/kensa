//! The trap site must be found perfectly: every planted defect, and nothing else.

use kensa::{crawl, fixture, rules, score, serve};

fn run(seed: u64) -> score::Score {
    let dir = std::env::temp_dir().join(format!("kensa-test-{}-{seed}", std::process::id()));
    let key = fixture::write(&dir, seed).unwrap();
    let origin = serve::Server::bind(&dir.join("site"), 0).unwrap().spawn();
    let site = crawl::crawl(&format!("{origin}/"), &crawl::Options::default()).unwrap();
    assert_eq!(site.pages.len(), 41, "every page is reached");
    let s = score::score(&rules::check(&site), &key);
    let _ = std::fs::remove_dir_all(&dir);
    s
}

#[test]
fn every_planted_defect_is_found_and_nothing_else() {
    for seed in 1..=25 {
        let s = run(seed);
        assert!(s.missed.is_empty() && s.extra.is_empty(), "seed {seed}: missed {:?}, extra {:?}", s.missed, s.extra);
    }
}

#[test]
fn seeds_move_the_defects() {
    let a = fixture::plan(1).key.expected;
    let b = fixture::plan(2).key.expected;
    assert_eq!(a.len(), b.len());
    assert_ne!(a, b);
}

#[test]
fn every_planted_rule_exists_and_is_scored() {
    for e in fixture::plan(3).key.expected {
        let r = rules::rule(&e.rule);
        assert_ne!(r.severity, rules::Severity::Notice, "{} is a notice, so it would never be scored", e.rule);
    }
}

#[test]
fn rules_doc_is_up_to_date() {
    let doc = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/docs/RULES.md")).unwrap();
    assert_eq!(doc, kensa::report::rules_markdown(), "run: kensa rules --markdown > docs/RULES.md");
}
