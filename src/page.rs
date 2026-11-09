//! What Kensa reads from one HTML page.

use scraper::{ElementRef, Html, Node, Selector};
use url::Url;

#[derive(Debug, Clone, Default)]
pub struct Page {
    pub url: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub h1_count: usize,
    pub html_lang: Option<String>,
    pub meta_charset: Option<String>,
    pub canonical: Option<String>,
    /// (code, absolute target, was the href absolute in the source)
    pub hreflang: Vec<(String, String, bool)>,
    pub links: Vec<String>,
    pub images_without_alt: usize,
    pub jsonld: Vec<String>,
    pub text: String,
    pub scripts: usize,
    pub noindex: bool,
}

fn sel(s: &str) -> Selector {
    Selector::parse(s).expect("valid selector")
}

fn clean(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Resolve an href against the page URL and drop the fragment.
pub fn absolutize(base: &Url, href: &str) -> Option<String> {
    let mut u = base.join(href.trim()).ok()?;
    if !matches!(u.scheme(), "http" | "https") {
        return None;
    }
    u.set_fragment(None);
    Some(u.to_string())
}

impl Page {
    pub fn parse(url: &str, html: &str, x_robots: &str) -> Page {
        let doc = Html::parse_document(html);
        let base = Url::parse(url).expect("crawled URLs are valid");
        let first_attr = |s: &str, attr: &str| doc.select(&sel(s)).next().and_then(|e| e.value().attr(attr)).map(clean);
        let mut page = Page {
            url: url.to_string(),
            title: doc.select(&sel("title")).next().map(|t| clean(&t.text().collect::<String>())),
            description: first_attr("meta[name=description]", "content"),
            h1_count: doc.select(&sel("h1")).count(),
            html_lang: first_attr("html", "lang").filter(|l| !l.is_empty()),
            meta_charset: first_attr("meta[charset]", "charset").or_else(|| {
                first_attr("meta[http-equiv]", "content")
                    .and_then(|c| c.to_ascii_lowercase().split("charset=").nth(1).map(|s| s.trim().to_string()))
            }),
            images_without_alt: doc.select(&sel("img")).filter(|i| i.value().attr("alt").is_none()).count(),
            scripts: doc
                .select(&sel("script"))
                .filter(|s| s.value().attr("type") != Some("application/ld+json"))
                .count(),
            ..Page::default()
        };
        page.canonical = doc
            .select(&sel("link[rel=canonical]"))
            .next()
            .and_then(|e| e.value().attr("href"))
            .and_then(|h| absolutize(&base, h));
        for e in doc.select(&sel("link[rel=alternate][hreflang]")) {
            let (Some(code), Some(href)) = (e.value().attr("hreflang"), e.value().attr("href")) else { continue };
            if let Some(abs) = absolutize(&base, href) {
                let was_absolute = Url::parse(href.trim()).is_ok();
                page.hreflang.push((code.trim().to_string(), abs, was_absolute));
            }
        }
        page.links = doc
            .select(&sel("a[href]"))
            .filter_map(|a| a.value().attr("href"))
            .filter_map(|h| absolutize(&base, h))
            .collect();
        page.jsonld =
            doc.select(&sel("script[type='application/ld+json']")).map(|s| s.text().collect::<String>()).collect();
        let robots_meta: String = doc
            .select(&sel("meta[name=robots]"))
            .filter_map(|m| m.value().attr("content"))
            .collect::<Vec<_>>()
            .join(",");
        page.noindex = format!("{robots_meta},{x_robots}").to_ascii_lowercase().contains("noindex");
        if let Some(body) = doc.select(&sel("body")).next() {
            page.text = visible_text(body);
        }
        page
    }
}

/// Text a crawler that does not run JavaScript would see.
fn visible_text(body: ElementRef) -> String {
    let mut out = String::new();
    for node in body.descendants() {
        if let Node::Text(t) = node.value() {
            let hidden = node.ancestors().any(|a| {
                a.value()
                    .as_element()
                    .map(|e| matches!(e.name(), "script" | "style" | "noscript" | "template"))
                    .unwrap_or(false)
            });
            if !hidden {
                out.push_str(t);
                out.push(' ');
            }
        }
    }
    clean(&out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_head_and_body() {
        let html = r#"<html lang="ja"><head><meta charset="utf-8"><title> 麻の便箋 </title>
            <link rel="canonical" href="/ja/a.html"><link rel="alternate" hreflang="fr" href="https://x.example/fr/a.html">
            <meta name="robots" content="NOINDEX"></head><body><h1>便箋</h1><img src="a.jpg"><img src="b.jpg" alt="">
            <script>var x = "hidden";</script><a href="b.html#top">b</a></body></html>"#;
        let p = Page::parse("https://x.example/ja/a.html", html, "");
        assert_eq!(p.title.as_deref(), Some("麻の便箋"));
        assert_eq!(p.canonical.as_deref(), Some("https://x.example/ja/a.html"));
        assert_eq!(p.hreflang[0].0, "fr");
        assert!(p.noindex);
        assert_eq!(p.images_without_alt, 1);
        assert_eq!(p.links, vec!["https://x.example/ja/b.html".to_string()]);
        assert!(!p.text.contains("hidden"));
        assert_eq!(p.meta_charset.as_deref(), Some("utf-8"));
    }
}
