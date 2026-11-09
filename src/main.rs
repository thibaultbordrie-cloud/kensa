use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::Context;
use clap::{Parser, Subcommand, ValueEnum};
use kensa::{crawl, fixture, report, rules, score, serve};
use serde::{Deserialize, Serialize};

#[derive(Parser)]
#[command(name = "kensa", version, about = "検査 Multilingual and AI-search SEO auditor (French, Japanese, Chinese)")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Crawl a site and report what is wrong.
    Audit {
        /// Start URL, for example https://www.example.com/
        url: String,
        #[arg(long, value_enum, default_value_t = Format::Text)]
        format: Format,
        /// Write the report to a file instead of the terminal.
        #[arg(long)]
        out: Option<PathBuf>,
        #[arg(long, default_value_t = 500)]
        max_pages: usize,
        /// Parallel requests. Keep it low on sites you do not own.
        #[arg(long, default_value_t = 4)]
        workers: usize,
        /// Exit with code 1 when a finding at this level or above exists (useful in CI).
        #[arg(long, value_enum, default_value_t = FailOn::Never)]
        fail_on: FailOn,
    },
    /// Write the trap site and its answer key.
    Fixture {
        #[arg(long, default_value = "kensa-fixture")]
        out: PathBuf,
        #[arg(long, default_value_t = 1)]
        seed: u64,
    },
    /// Serve a folder (the trap site) on 127.0.0.1.
    Serve {
        dir: PathBuf,
        #[arg(long, default_value_t = 8080)]
        port: u16,
    },
    /// Score a JSON report against an answer key.
    Score { report: PathBuf, key: PathBuf },
    /// Build the trap site, serve it, audit it and score the result, for one or several seeds.
    Bench {
        #[arg(long, default_value_t = 1)]
        seed: u64,
        /// How many consecutive seeds to run.
        #[arg(long, default_value_t = 1)]
        runs: u64,
    },
    /// List the rules.
    Rules {
        #[arg(long)]
        markdown: bool,
    },
}

#[derive(Clone, Copy, ValueEnum)]
enum Format {
    Text,
    Json,
    Markdown,
}

#[derive(Clone, Copy, ValueEnum)]
enum FailOn {
    Error,
    Warning,
    Never,
}

#[derive(Serialize, Deserialize)]
struct JsonReport {
    origin: String,
    pages: usize,
    findings: Vec<rules::Finding>,
}

fn audit(url: &str, max_pages: usize, workers: usize) -> anyhow::Result<JsonReport> {
    let opts = crawl::Options { max_pages, workers, ..Default::default() };
    let site = crawl::crawl(url, &opts).with_context(|| format!("crawling {url}"))?;
    Ok(JsonReport { origin: site.origin.clone(), pages: site.pages.len(), findings: rules::check(&site) })
}

fn print_score(s: &score::Score) {
    let o = &s.overall;
    println!(
        "precision {:.3} · recall {:.3} · F1 {:.3}  ({} found, {} false alarms, {} missed)",
        o.precision(),
        o.recall(),
        o.f1(),
        o.true_positives,
        o.false_positives,
        o.false_negatives
    );
    for (rule, url) in &s.missed {
        println!("  missed      {rule:<28} {url}");
    }
    for (rule, url) in &s.extra {
        println!("  false alarm {rule:<28} {url}");
    }
}

fn run() -> anyhow::Result<ExitCode> {
    match Cli::parse().command {
        Command::Audit { url, format, out, max_pages, workers, fail_on } => {
            let r = audit(&url, max_pages, workers)?;
            let text = match format {
                Format::Text => report::text(&r.origin, r.pages, &r.findings),
                Format::Json => serde_json::to_string_pretty(&r)? + "\n",
                Format::Markdown => report::markdown(&r.origin, r.pages, &r.findings),
            };
            match out {
                Some(path) => std::fs::write(&path, text).with_context(|| format!("writing {}", path.display()))?,
                None => print!("{text}"),
            }
            let worst = r.findings.iter().map(|f| f.severity).min();
            let fail = matches!(
                (fail_on, worst),
                (FailOn::Error, Some(rules::Severity::Error))
                    | (FailOn::Warning, Some(rules::Severity::Error | rules::Severity::Warning))
            );
            Ok(if fail { ExitCode::FAILURE } else { ExitCode::SUCCESS })
        }
        Command::Fixture { out, seed } => {
            let key = fixture::write(&out, seed)?;
            println!(
                "Trap site written to {}/site with {} planted defects (answer key: {}/answer-key.json).",
                out.display(),
                key.expected.len(),
                out.display()
            );
            println!("Next: kensa serve {}/site, then kensa audit http://127.0.0.1:8080/", out.display());
            Ok(ExitCode::SUCCESS)
        }
        Command::Serve { dir, port } => {
            let server = serve::Server::bind(&dir, port)?;
            println!("Serving {} on {}/ (Ctrl+C to stop)", dir.display(), server.origin);
            server.run();
            Ok(ExitCode::SUCCESS)
        }
        Command::Score { report, key } => {
            let r: JsonReport =
                serde_json::from_str(&std::fs::read_to_string(&report)?).context("reading the report")?;
            let k: score::AnswerKey =
                serde_json::from_str(&std::fs::read_to_string(&key)?).context("reading the answer key")?;
            print_score(&score::score(&r.findings, &k));
            Ok(ExitCode::SUCCESS)
        }
        Command::Bench { seed, runs } => {
            let mut perfect = true;
            for s in seed..seed + runs {
                let dir = std::env::temp_dir().join(format!("kensa-bench-{}-{s}", std::process::id()));
                let key = fixture::write(&dir, s)?;
                let origin = serve::Server::bind(&dir.join("site"), 0)?.spawn();
                let r = audit(&format!("{origin}/"), 500, 8)?;
                let sc = score::score(&r.findings, &key);
                print!("seed {s:>4} · {} pages · {} planted · ", r.pages, key.expected.len());
                print_score(&sc);
                perfect &= sc.overall.f1() == 1.0;
                let _ = std::fs::remove_dir_all(&dir);
            }
            Ok(if perfect { ExitCode::SUCCESS } else { ExitCode::FAILURE })
        }
        Command::Rules { markdown } => {
            if markdown {
                print!("{}", report::rules_markdown());
            } else {
                for r in rules::RULES {
                    println!(
                        "{:<28} {:<8} {:<10} {}",
                        r.id,
                        format!("{:?}", r.severity).to_lowercase(),
                        r.category,
                        r.summary
                    );
                }
            }
            Ok(ExitCode::SUCCESS)
        }
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(e) => {
            eprintln!("kensa: {e:#}");
            ExitCode::from(2)
        }
    }
}
