//! robots.txt parsing and matching, following RFC 9309 (longest match wins, Allow wins ties).

#[derive(Debug, Default, Clone)]
pub struct Robots {
    groups: Vec<Group>,
    pub sitemaps: Vec<String>,
}

#[derive(Debug, Default, Clone)]
struct Group {
    agents: Vec<String>,
    rules: Vec<(bool, String)>, // (allow, pattern)
}

/// AI crawlers that matter for visibility in AI answers and search, by owner.
/// `search = true` means the crawler fetches pages to answer users or cite sources;
/// blocking it removes the site from those answers. `search = false` means a training crawler:
/// blocking it is a legitimate choice, reported as a notice only.
pub const AI_CRAWLERS: &[(&str, &str, bool)] = &[
    ("OAI-SearchBot", "OpenAI", true),
    ("ChatGPT-User", "OpenAI", true),
    ("Claude-SearchBot", "Anthropic", true),
    ("Claude-User", "Anthropic", true),
    ("PerplexityBot", "Perplexity", true),
    ("Perplexity-User", "Perplexity", true),
    ("GPTBot", "OpenAI", false),
    ("ClaudeBot", "Anthropic", false),
    ("Google-Extended", "Google", false),
    ("Applebot-Extended", "Apple", false),
    ("CCBot", "Common Crawl", false),
];

impl Robots {
    pub fn parse(text: &str) -> Robots {
        let mut robots = Robots::default();
        let mut current: Option<Group> = None;
        let mut last_was_agent = false;
        for raw in text.lines() {
            let line = raw.split('#').next().unwrap_or("").trim();
            let Some((key, value)) = line.split_once(':') else { continue };
            let (key, value) = (key.trim().to_ascii_lowercase(), value.trim().to_string());
            match key.as_str() {
                "user-agent" => {
                    if !last_was_agent {
                        if let Some(g) = current.take() {
                            robots.groups.push(g);
                        }
                        current = Some(Group::default());
                    }
                    if let Some(g) = current.as_mut() {
                        g.agents.push(value.to_ascii_lowercase());
                    }
                    last_was_agent = true;
                }
                "allow" | "disallow" => {
                    if let Some(g) = current.as_mut() {
                        if !value.is_empty() {
                            g.rules.push((key == "allow", value));
                        }
                    }
                    last_was_agent = false;
                }
                "sitemap" => robots.sitemaps.push(value),
                _ => last_was_agent = false,
            }
        }
        if let Some(g) = current {
            robots.groups.push(g);
        }
        robots
    }

    /// Is `path` (path plus query) allowed for the crawler whose product token is `agent`?
    pub fn allowed(&self, agent: &str, path: &str) -> bool {
        let agent = agent.to_ascii_lowercase();
        let mut groups: Vec<&Group> = self.groups.iter().filter(|g| g.agents.contains(&agent)).collect();
        if groups.is_empty() {
            groups = self.groups.iter().filter(|g| g.agents.iter().any(|a| a == "*")).collect();
        }
        let mut best: Option<(usize, bool)> = None;
        for (allow, pattern) in groups.iter().flat_map(|g| g.rules.iter()) {
            if matches(pattern, path) {
                let len = pattern.len();
                best = match best {
                    Some((l, a)) if l > len || (l == len && a) => Some((l, a)),
                    _ => Some((len, *allow)),
                };
            }
        }
        best.map(|(_, allow)| allow).unwrap_or(true)
    }
}

/// Pattern match with `*` (any run of characters) and a trailing `$` (end of path).
fn matches(pattern: &str, path: &str) -> bool {
    let (pattern, anchored) = match pattern.strip_suffix('$') {
        Some(p) => (p, true),
        None => (pattern, false),
    };
    glob(pattern.as_bytes(), path.as_bytes(), anchored)
}

fn glob(p: &[u8], s: &[u8], anchored: bool) -> bool {
    match p.first() {
        None => !anchored || s.is_empty(),
        Some(b'*') => (0..=s.len()).any(|i| glob(&p[1..], &s[i..], anchored)),
        Some(c) => s.first() == Some(c) && glob(&p[1..], &s[1..], anchored),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TXT: &str = "User-agent: *\nDisallow: /cart\nAllow: /cart/help\n\nUser-agent: GPTBot\nUser-agent: CCBot\nDisallow: /\n\nUser-agent: OAI-SearchBot\nDisallow: /ja/\nSitemap: https://example.com/sitemap.xml\n";

    #[test]
    fn longest_match_wins() {
        let r = Robots::parse(TXT);
        assert!(!r.allowed("kensa", "/cart/checkout"));
        assert!(r.allowed("kensa", "/cart/help"));
        assert!(r.allowed("kensa", "/fr/"));
    }

    #[test]
    fn grouped_agents_share_rules() {
        let r = Robots::parse(TXT);
        assert!(!r.allowed("GPTBot", "/fr/"));
        assert!(!r.allowed("ccbot", "/"));
        assert!(!r.allowed("OAI-SearchBot", "/ja/index.html"));
        assert!(r.allowed("OAI-SearchBot", "/fr/"));
        assert_eq!(r.sitemaps.len(), 1);
    }

    #[test]
    fn wildcards_and_anchors() {
        assert!(matches("/*.pdf$", "/docs/a.pdf"));
        assert!(!matches("/*.pdf$", "/docs/a.pdf?x=1"));
        assert!(matches("/fr/*/old", "/fr/a/b/old-page"));
        assert!(matches("/index.html$", "/index.html"));
        assert!(!matches("/index.html$", "/index.html2"));
    }
}
