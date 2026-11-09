//! The checks. Each finding names a rule, a severity and the URL to fix.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use unicode_width::UnicodeWidthStr;

use crate::crawl::Site;
use crate::lang::{self, Profile};
use crate::page::Page;
use crate::robots::AI_CRAWLERS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
    Notice,
}

pub struct Rule {
    pub id: &'static str,
    pub severity: Severity,
    pub category: &'static str,
    pub summary: &'static str,
}

const fn r(id: &'static str, severity: Severity, category: &'static str, summary: &'static str) -> Rule {
    Rule { id, severity, category, summary }
}

use Severity::{Error as E, Notice as N, Warning as W};

pub const RULES: &[Rule] = &[
    r("title-missing", E, "content", "The page has no <title>."),
    r("title-too-wide", W, "content", "The title is wider than 60 columns. CJK characters count double, so 31 Japanese characters are already too long."),
    r("title-duplicate", W, "content", "Several indexable pages share the same title."),
    r("title-fullwidth-ascii", N, "content", "The title uses full-width Latin letters or digits (ＳＡＬＥ instead of SALE)."),
    r("description-missing", W, "content", "The page has no meta description."),
    r("description-too-wide", N, "content", "The meta description is wider than 160 columns (CJK characters count double)."),
    r("h1-missing", W, "content", "The page has no <h1>."),
    r("h1-multiple", N, "content", "The page has several <h1>."),
    r("img-alt-missing", W, "content", "An image has no alt attribute (alt=\"\" is fine for decoration)."),
    r("content-js-only", E, "ai-search", "The HTML holds almost no text: the content is drawn by JavaScript, which AI crawlers do not run."),
    r("html-lang-missing", W, "language", "The <html> element has no lang attribute."),
    r("html-lang-invalid", W, "language", "The lang attribute is not a valid language tag."),
    r("html-lang-mismatch", E, "language", "The declared language is not the language of the text (for example lang=\"ja\" on an untranslated French page)."),
    r("zh-script-mismatch", E, "language", "The page declares Traditional Chinese (zh-Hant, zh-TW, zh-HK) but is written in Simplified, or the reverse."),
    r("charset-not-utf8", W, "language", "The page declares an encoding other than UTF-8 (Shift_JIS, Big5, GB2312…)."),
    r("hreflang-invalid-code", E, "hreflang", "A hreflang value is not a valid code (jp instead of ja, en-UK instead of en-GB…)."),
    r("hreflang-relative-url", W, "hreflang", "A hreflang link uses a relative URL; it must be absolute."),
    r("hreflang-no-self", W, "hreflang", "The page lists its alternates but not itself."),
    r("hreflang-no-return", E, "hreflang", "Another page names this page as an alternate, but this page does not name it back."),
    r("hreflang-target-error", E, "hreflang", "A hreflang link points to a URL that does not answer 200 (error or redirect)."),
    r("hreflang-duplicate-code", E, "hreflang", "The same hreflang code points to two different URLs."),
    r("hreflang-lang-mismatch", W, "hreflang", "A hreflang code does not match the language or Chinese script of the page it points to."),
    r("hreflang-no-x-default", N, "hreflang", "The hreflang set has no x-default."),
    r("canonical-missing", N, "indexing", "The page has no canonical link."),
    r("canonical-error", E, "indexing", "The canonical URL does not answer 200."),
    r("canonical-other-language", E, "indexing", "The canonical points to a page in another language, which cancels hreflang."),
    r("link-broken", E, "indexing", "An internal link leads to an error."),
    r("redirect-chain", W, "indexing", "An internal link goes through two redirects or more."),
    r("sitemap-missing", W, "indexing", "No sitemap was found (robots.txt Sitemap line or /sitemap.xml)."),
    r("sitemap-url-error", E, "indexing", "The sitemap lists a URL that does not answer 200."),
    r("sitemap-url-noindex", W, "indexing", "The sitemap lists a page marked noindex."),
    r("ai-search-blocked", E, "ai-search", "robots.txt blocks a crawler that AI assistants use to find and cite pages (OAI-SearchBot, Claude-SearchBot, PerplexityBot…)."),
    r("ai-training-blocked", N, "ai-search", "robots.txt blocks an AI training crawler (GPTBot, ClaudeBot, Google-Extended…). A legitimate choice, listed for information."),
    r("llms-txt-missing", N, "ai-search", "There is no /llms.txt."),
    r("llms-txt-invalid", W, "ai-search", "/llms.txt does not start with a # title, or links to pages that fail."),
    r("jsonld-invalid", E, "ai-search", "A JSON-LD block is not valid JSON."),
    r("jsonld-product-incomplete", W, "ai-search", "A Product has no name, or an offer without price or priceCurrency."),
    r("jsonld-organization-missing", N, "ai-search", "No page describes the Organization in JSON-LD."),
];

pub fn rule(id: &str) -> &'static Rule {
    RULES.iter().find(|r| r.id == id).unwrap_or_else(|| panic!("unknown rule {id}"))
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct Finding {
    pub rule: String,
    pub severity: Severity,
    pub url: String,
    pub message: String,
}

struct Out<'a> {
    site: &'a Site,
    found: BTreeMap<(String, String), Finding>,
}

impl Out<'_> {
    fn add(&mut self, id: &str, url: &str, message: impl Into<String>) {
        let path = self.site.path(url);
        let r = rule(id);
        self.found.entry((id.to_string(), path.clone())).or_insert(Finding {
            rule: id.to_string(),
            severity: r.severity,
            url: path,
            message: message.into(),
        });
    }
}

/// What Kensa concluded about the language of one page.
struct LangView {
    declared: Option<lang::Tag>,
    profile: Profile,
    js_only: bool,
    /// Declared language agrees with the text (or there is too little text to tell).
    consistent: bool,
}

fn lang_view(page: &Page) -> LangView {
    let profile = Profile::of(&page.text);
    let declared = page.html_lang.as_deref().and_then(|l| lang::parse(l).ok());
    let js_only = profile.letters() < 40 && page.scripts > 0;
    let mut consistent = declared.is_some();
    if let (Some(tag), Some(guess)) = (&declared, profile.guess()) {
        consistent = lang::agrees(&tag.language, guess);
        if consistent && tag.language == "zh" {
            if let (Some(want), Some(have)) = (lang::zh_script(tag), profile.zh_script()) {
                consistent = want == have;
            }
        }
    }
    LangView { declared, profile, js_only, consistent }
}

fn indexable(page: &Page) -> bool {
    !page.noindex && page.canonical.as_deref().map(|c| c == page.url).unwrap_or(true)
}

pub fn check(site: &Site) -> Vec<Finding> {
    let mut out = Out { site, found: BTreeMap::new() };
    let views: BTreeMap<&str, LangView> = site.pages.iter().map(|(u, p)| (u.as_str(), lang_view(p))).collect();
    for (url, page) in &site.pages {
        content(&mut out, page, &views[url.as_str()]);
        language(&mut out, page, &views[url.as_str()]);
        links(&mut out, site, page, &views);
        hreflang(&mut out, site, page, &views);
    }
    duplicates(&mut out, site);
    sitemap(&mut out, site);
    ai_search(&mut out, site);
    let mut v: Vec<Finding> = out.found.into_values().collect();
    v.sort_by(|a, b| (a.severity, &a.url, &a.rule).cmp(&(b.severity, &b.url, &b.rule)));
    v
}

fn content(out: &mut Out, page: &Page, view: &LangView) {
    let u = &page.url;
    match page.title.as_deref() {
        None | Some("") => out.add("title-missing", u, "no <title>"),
        Some(t) => {
            let w = t.width();
            if w > 60 {
                out.add("title-too-wide", u, format!("{w} columns ({} characters): \"{t}\"", t.chars().count()));
            }
            if t.chars().any(|c| c.is_alphanumeric() && ('\u{FF10}'..='\u{FF5A}').contains(&c)) {
                out.add("title-fullwidth-ascii", u, format!("\"{t}\""));
            }
        }
    }
    match page.description.as_deref() {
        None | Some("") => {
            if !page.noindex {
                out.add("description-missing", u, "no meta description")
            }
        }
        Some(d) if d.width() > 160 => out.add("description-too-wide", u, format!("{} columns", d.width())),
        _ => {}
    }
    if view.js_only {
        out.add(
            "content-js-only",
            u,
            format!("{} letters of text without JavaScript, {} script(s)", view.profile.letters(), page.scripts),
        );
        return;
    }
    match page.h1_count {
        0 => out.add("h1-missing", u, "no <h1>"),
        1 => {}
        n => out.add("h1-multiple", u, format!("{n} <h1>")),
    }
    if page.images_without_alt > 0 {
        out.add("img-alt-missing", u, format!("{} image(s) without alt", page.images_without_alt));
    }
}

fn language(out: &mut Out, page: &Page, view: &LangView) {
    let u = &page.url;
    let charset = page.meta_charset.clone().or_else(|| {
        out.site
            .content_types
            .get(u)
            .and_then(|ct| ct.to_ascii_lowercase().split("charset=").nth(1).map(|s| s.trim().to_string()))
    });
    if let Some(cs) = charset {
        if !matches!(cs.to_ascii_lowercase().as_str(), "utf-8" | "utf8") {
            out.add("charset-not-utf8", u, format!("declared charset {cs}"));
        }
    }
    let Some(raw) = page.html_lang.as_deref() else {
        out.add("html-lang-missing", u, "<html> has no lang");
        return;
    };
    let Some(tag) = &view.declared else {
        let why = lang::parse(raw).err().unwrap_or_default();
        out.add("html-lang-invalid", u, why);
        return;
    };
    if view.js_only || view.consistent {
        return;
    }
    let p = &view.profile;
    let guess = p.guess().unwrap_or("?");
    if tag.language == "zh" && guess == "zh" {
        out.add(
            "zh-script-mismatch",
            u,
            format!(
                "lang=\"{raw}\" but the text has {} Simplified-only and {} Traditional-only characters",
                p.simplified, p.traditional
            ),
        );
    } else {
        out.add(
            "html-lang-mismatch",
            u,
            format!(
                "lang=\"{raw}\" but the text looks {} (latin {}, kana {}, han {})",
                if guess == "latin" { "Latin-script" } else { guess },
                p.latin,
                p.kana,
                p.han
            ),
        );
    }
}

fn links(out: &mut Out, site: &Site, page: &Page, views: &BTreeMap<&str, LangView>) {
    let u = &page.url;
    for link in &page.links {
        let Some(res) = site.resolutions.get(link) else { continue };
        if res.status >= 400 || res.status == 0 {
            out.add("link-broken", u, format!("{} answers {}", site.path(link), res.status));
        } else if res.hops.len() >= 2 {
            let chain: Vec<String> = res.hops.iter().chain([&res.final_url]).map(|h| site.path(h)).collect();
            out.add("redirect-chain", u, chain.join(" → "));
        }
    }
    match page.canonical.as_deref() {
        None => out.add("canonical-missing", u, "no canonical link"),
        Some(c) if c != u.as_str() => match site.resolutions.get(c) {
            Some(r) if r.status == 200 && r.hops.is_empty() => {
                let mine = views.get(u.as_str()).and_then(|v| v.declared.as_ref()).map(|t| t.language.clone());
                let theirs = views.get(c).and_then(|v| v.declared.as_ref()).map(|t| t.language.clone());
                if let (Some(a), Some(b)) = (mine, theirs) {
                    if a != b {
                        out.add(
                            "canonical-other-language",
                            u,
                            format!("a {a} page declares a {b} page as canonical: {}", site.path(c)),
                        );
                    }
                }
            }
            Some(r) => {
                let what = if r.hops.is_empty() { format!("answers {}", r.status) } else { "redirects".into() };
                out.add("canonical-error", u, format!("canonical {} {what}", site.path(c)));
            }
            None => {}
        },
        _ => {}
    }
}

fn hreflang(out: &mut Out, site: &Site, page: &Page, views: &BTreeMap<&str, LangView>) {
    let u = &page.url;
    if page.hreflang.is_empty() {
        return;
    }
    let mut by_code: BTreeMap<String, BTreeSet<&str>> = BTreeMap::new();
    let mut has_self = false;
    let mut has_default = false;
    for (code, target, absolute) in &page.hreflang {
        by_code.entry(code.to_ascii_lowercase()).or_default().insert(target);
        has_self |= target == u;
        if !absolute {
            out.add("hreflang-relative-url", u, format!("hreflang=\"{code}\" uses a relative URL"));
        }
        let res = site.resolutions.get(target);
        match res {
            Some(r) if r.status != 200 || !r.hops.is_empty() => {
                let what = if r.hops.is_empty() { format!("answers {}", r.status) } else { "redirects".to_string() };
                out.add("hreflang-target-error", u, format!("hreflang=\"{code}\" → {} {what}", site.path(target)));
            }
            _ => {}
        }
        if code.eq_ignore_ascii_case("x-default") {
            has_default = true;
            continue;
        }
        let tag = match lang::parse(code) {
            Ok(t) => t,
            Err(why) => {
                out.add("hreflang-invalid-code", u, why);
                continue;
            }
        };
        // Blame the annotation only when the target page is itself consistent.
        if let Some(v) = views.get(target.as_str()).filter(|v| v.consistent && !v.js_only) {
            let Some(theirs) = &v.declared else { continue };
            let script_clash = match (lang::zh_script(&tag), v.profile.zh_script().or(lang::zh_script(theirs))) {
                (Some(a), Some(b)) => a != b,
                _ => false,
            };
            if theirs.language != tag.language || script_clash {
                out.add(
                    "hreflang-lang-mismatch",
                    u,
                    format!(
                        "hreflang=\"{code}\" points to {}, a page in {}",
                        site.path(target),
                        page_lang_name(theirs, v)
                    ),
                );
            }
        }
    }
    for (code, targets) in &by_code {
        if targets.len() > 1 {
            out.add("hreflang-duplicate-code", u, format!("hreflang=\"{code}\" points to {} URLs", targets.len()));
        }
    }
    if !has_self {
        out.add("hreflang-no-self", u, "the page is missing from its own hreflang set");
    }
    if !has_default {
        out.add("hreflang-no-x-default", u, "no x-default");
    }
    // Return links: every alternate that names this page must be named back by it.
    let mine: BTreeSet<&str> = page.hreflang.iter().map(|h| h.1.as_str()).collect();
    for (other_url, other) in &site.pages {
        if other_url == u || mine.contains(other_url.as_str()) {
            continue;
        }
        if other.hreflang.iter().any(|h| h.1 == *u && !h.0.eq_ignore_ascii_case("x-default")) {
            out.add(
                "hreflang-no-return",
                u,
                format!(
                    "{} names this page as an alternate, but this page does not name it back",
                    site.path(other_url)
                ),
            );
        }
    }
}

fn page_lang_name(tag: &lang::Tag, v: &LangView) -> String {
    match v.profile.zh_script().or(lang::zh_script(tag)) {
        Some(lang::ZhScript::Traditional) => "Traditional Chinese".into(),
        Some(lang::ZhScript::Simplified) => "Simplified Chinese".into(),
        None => format!("\"{}\"", tag.language),
    }
}

fn duplicates(out: &mut Out, site: &Site) {
    let mut by_title: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for (u, p) in &site.pages {
        if let Some(t) = p.title.as_deref().filter(|t| !t.is_empty() && indexable(p)) {
            by_title.entry(t).or_default().push(u);
        }
    }
    for (title, urls) in by_title.into_iter().filter(|(_, v)| v.len() > 1) {
        for u in &urls {
            out.add("title-duplicate", u, format!("\"{title}\" is used by {} pages", urls.len()));
        }
    }
}

fn sitemap(out: &mut Out, site: &Site) {
    if !site.sitemap_found {
        let at = format!("{}/sitemap.xml", site.origin);
        out.add("sitemap-missing", &at, "no sitemap in robots.txt and no /sitemap.xml");
        return;
    }
    for u in &site.sitemap_urls {
        match site.resolutions.get(u) {
            Some(r) if r.status != 200 || !r.hops.is_empty() => {
                let what = if r.hops.is_empty() { format!("answers {}", r.status) } else { "redirects".into() };
                out.add("sitemap-url-error", u, format!("listed in the sitemap but {what}"));
            }
            Some(_) if site.pages.get(u).map(|p| p.noindex).unwrap_or(false) => {
                out.add("sitemap-url-noindex", u, "listed in the sitemap but marked noindex");
            }
            _ => {}
        }
    }
}

fn ai_search(out: &mut Out, site: &Site) {
    let robots_url = format!("{}/robots.txt", site.origin);
    if site.robots_txt.is_some() {
        // Sample paths: the root and every first-level folder that holds crawled pages.
        let mut paths: BTreeSet<String> = BTreeSet::from(["/".to_string()]);
        for u in site.pages.keys() {
            let p = site.path(u);
            if let Some(dir) =
                p.trim_start_matches('/').split('/').next().filter(|d| !d.is_empty() && p.matches('/').count() > 1)
            {
                paths.insert(format!("/{dir}/"));
            }
        }
        let mut search = Vec::new();
        let mut training = Vec::new();
        for (bot, owner, is_search) in AI_CRAWLERS {
            let blocked: Vec<&str> =
                paths.iter().filter(|p| !site.robots.allowed(bot, p)).map(|p| p.as_str()).collect();
            if !blocked.is_empty() {
                let line = format!("{bot} ({owner}): {}", blocked.join(", "));
                if *is_search {
                    search.push(line)
                } else {
                    training.push(line)
                }
            }
        }
        if !search.is_empty() {
            out.add("ai-search-blocked", &robots_url, search.join("; "));
        }
        if !training.is_empty() {
            out.add("ai-training-blocked", &robots_url, training.join("; "));
        }
    }
    let llms_url = format!("{}/llms.txt", site.origin);
    match site.llms_txt.as_deref() {
        None => out.add("llms-txt-missing", &llms_url, "no /llms.txt"),
        Some(text) => {
            let mut problems = Vec::new();
            if !text.lines().find(|l| !l.trim().is_empty()).map(|l| l.starts_with("# ")).unwrap_or(false) {
                problems.push("does not start with a \"# \" title".to_string());
            }
            let base = url::Url::parse(&site.origin).expect("origin");
            for link in crate::crawl::llms_links(&base, text) {
                if let Some(r) = site.resolutions.get(&link).filter(|r| r.status != 200) {
                    problems.push(format!("{} answers {}", site.path(&link), r.status));
                }
            }
            if !problems.is_empty() {
                out.add("llms-txt-invalid", &llms_url, problems.join("; "));
            }
        }
    }
    let mut organization = false;
    for (u, page) in &site.pages {
        for block in &page.jsonld {
            match serde_json::from_str::<Value>(block) {
                Err(e) => out.add("jsonld-invalid", u, format!("JSON error: {e}")),
                Ok(v) => {
                    let mut nodes = Vec::new();
                    collect(&v, &mut nodes);
                    for n in nodes {
                        if is_type(n, "Organization") || is_type(n, "OnlineStore") {
                            organization = true;
                        }
                        if is_type(n, "Product") {
                            if let Some(why) = product_gap(n) {
                                out.add("jsonld-product-incomplete", u, why);
                            }
                        }
                    }
                }
            }
        }
    }
    if !organization && !site.pages.is_empty() {
        let home = format!("{}/", site.origin);
        out.add("jsonld-organization-missing", &home, "no Organization in any JSON-LD block");
    }
}

/// Every object of a JSON-LD document, including those inside @graph and nested values.
fn collect<'a>(v: &'a Value, out: &mut Vec<&'a Value>) {
    match v {
        Value::Object(map) => {
            out.push(v);
            map.values().for_each(|x| collect(x, out));
        }
        Value::Array(items) => items.iter().for_each(|x| collect(x, out)),
        _ => {}
    }
}

fn is_type(v: &Value, name: &str) -> bool {
    match v.get("@type") {
        Some(Value::String(s)) => s == name,
        Some(Value::Array(a)) => a.iter().any(|x| x.as_str() == Some(name)),
        _ => false,
    }
}

fn product_gap(p: &Value) -> Option<String> {
    if p.get("name").and_then(|n| n.as_str()).map(|s| s.trim().is_empty()).unwrap_or(true) {
        return Some("Product without name".into());
    }
    let offers: Vec<&Value> = match p.get("offers") {
        Some(Value::Array(a)) => a.iter().collect(),
        Some(o @ Value::Object(_)) => vec![o],
        _ => return Some("Product without offers".into()),
    };
    for o in offers {
        for field in ["price", "priceCurrency"] {
            if o.get(field).map(|x| x.is_null() || x.as_str() == Some("")).unwrap_or(true) {
                return Some(format!("offer without {field}"));
            }
        }
    }
    None
}
