//! Crawl one site: robots.txt, sitemaps, llms.txt, then pages breadth first, in parallel.

use std::collections::{BTreeMap, BTreeSet};

use url::Url;

use crate::fetch::{Fetcher, Response};
use crate::page::Page;
use crate::robots::Robots;

#[derive(Debug, Clone)]
pub struct Resolution {
    pub status: u16,
    /// URLs that answered with a redirect, in order (empty when the URL answered directly).
    pub hops: Vec<String>,
    pub final_url: String,
}

#[derive(Debug, Default)]
pub struct Site {
    pub origin: String,
    pub pages: BTreeMap<String, Page>,
    pub content_types: BTreeMap<String, String>,
    pub resolutions: BTreeMap<String, Resolution>,
    pub robots_txt: Option<String>,
    pub robots: Robots,
    pub sitemap_urls: Vec<String>,
    pub sitemap_found: bool,
    pub llms_txt: Option<String>,
}

pub struct Options {
    pub max_pages: usize,
    pub workers: usize,
    pub timeout_secs: u64,
}

impl Default for Options {
    fn default() -> Self {
        Options { max_pages: 500, workers: 4, timeout_secs: 15 }
    }
}

fn same_origin(a: &Url, b: &str) -> bool {
    Url::parse(b).map(|b| b.origin() == a.origin()).unwrap_or(false)
}

/// Extract every <loc> of a sitemap or sitemap index.
pub fn sitemap_locs(xml: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = xml;
    while let Some(start) = rest.find("<loc>") {
        rest = &rest[start + 5..];
        if let Some(end) = rest.find("</loc>") {
            out.push(rest[..end].trim().replace("&amp;", "&"));
            rest = &rest[end..];
        }
    }
    out
}

/// Absolute targets of the Markdown links in an llms.txt file.
pub fn llms_links(base: &Url, text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(i) = rest.find("](") {
        rest = &rest[i + 2..];
        if let Some(end) = rest.find(')') {
            if let Some(abs) = crate::page::absolutize(base, &rest[..end]) {
                out.push(abs);
            }
            rest = &rest[end..];
        }
    }
    out
}

/// GET a URL and follow up to 10 redirects by hand.
fn resolve(fetcher: &Fetcher, start: &str) -> (Resolution, Response) {
    let mut url = start.to_string();
    let mut hops = Vec::new();
    loop {
        let resp = fetcher.get(&url);
        let redirect = (300..400).contains(&resp.status) && resp.location.is_some() && hops.len() < 10;
        if !redirect {
            return (Resolution { status: resp.status, hops, final_url: url }, resp);
        }
        let next = Url::parse(&url).ok().and_then(|b| b.join(resp.location.as_deref().unwrap_or("")).ok());
        hops.push(url.clone());
        match next {
            Some(n) => url = n.to_string(),
            None => return (Resolution { status: resp.status, hops, final_url: url }, resp),
        }
    }
}

pub fn crawl(start: &str, opts: &Options) -> anyhow::Result<Site> {
    let start_url = Url::parse(start)?;
    let origin = start_url.origin().ascii_serialization();
    let fetcher = Fetcher::new(opts.timeout_secs)?;
    let mut site = Site { origin: origin.clone(), ..Site::default() };

    let first = fetcher.get(start);
    if first.status == 0 {
        anyhow::bail!("cannot reach {start}: {}", first.body);
    }
    let robots = fetcher.get(&format!("{origin}/robots.txt"));
    if robots.status == 200 {
        site.robots = Robots::parse(&robots.body);
        site.robots_txt = Some(robots.body);
    }
    let mut sitemaps: Vec<String> = site.robots.sitemaps.clone();
    if sitemaps.is_empty() {
        sitemaps.push(format!("{origin}/sitemap.xml"));
    }
    let mut seen_maps = BTreeSet::new();
    while let Some(map) = sitemaps.pop() {
        if !seen_maps.insert(map.clone()) || seen_maps.len() > 50 {
            continue;
        }
        let resp = fetcher.get(&map);
        if resp.status != 200 {
            continue;
        }
        site.sitemap_found = true;
        let locs = sitemap_locs(&resp.body);
        if resp.body.contains("<sitemapindex") {
            sitemaps.extend(locs);
        } else {
            site.sitemap_urls.extend(locs.into_iter().filter(|l| same_origin(&start_url, l)));
        }
    }
    let llms = fetcher.get(&format!("{origin}/llms.txt"));
    let mut extra: Vec<String> = Vec::new();
    if llms.status == 200 {
        extra = llms_links(&start_url, &llms.body);
        site.llms_txt = Some(llms.body);
    }

    let mut queued: BTreeSet<String> = BTreeSet::new();
    let mut frontier: Vec<String> = Vec::new();
    let push = |u: &str, queued: &mut BTreeSet<String>, frontier: &mut Vec<String>| {
        if same_origin(&start_url, u) && queued.len() < opts.max_pages * 4 && queued.insert(u.to_string()) {
            frontier.push(u.to_string());
        }
    };
    push(start, &mut queued, &mut frontier);
    for u in site.sitemap_urls.clone().into_iter().chain(extra) {
        push(&u, &mut queued, &mut frontier);
    }

    while !frontier.is_empty() && site.pages.len() < opts.max_pages {
        let batch: Vec<String> = std::mem::take(&mut frontier)
            .into_iter()
            .filter(|u| {
                let path = Url::parse(u).map(|p| p[url::Position::BeforePath..].to_string()).unwrap_or_default();
                site.robots.allowed("kensa", &path)
            })
            .collect();
        let chunk = batch.len().div_ceil(opts.workers.max(1)).max(1);
        let results: Vec<(String, Resolution, Response)> = std::thread::scope(|s| {
            let handles: Vec<_> = batch
                .chunks(chunk)
                .map(|part| {
                    let fetcher = &fetcher;
                    s.spawn(move || {
                        part.iter()
                            .map(|u| {
                                let (r, resp) = resolve(fetcher, u);
                                (u.clone(), r, resp)
                            })
                            .collect::<Vec<_>>()
                    })
                })
                .collect();
            handles.into_iter().flat_map(|h| h.join().unwrap_or_default()).collect()
        });
        for (url, res, resp) in results {
            let final_url = res.final_url.clone();
            site.resolutions.insert(url, res.clone());
            let html = resp.content_type.contains("text/html") || resp.body.trim_start().starts_with("<!");
            if res.status != 200 || !html || site.pages.contains_key(&final_url) || !same_origin(&start_url, &final_url)
            {
                continue;
            }
            if site.pages.len() >= opts.max_pages {
                break;
            }
            let page = Page::parse(&final_url, &resp.body, &resp.x_robots);
            let mut next: Vec<String> = page.links.clone();
            next.extend(page.hreflang.iter().map(|h| h.1.clone()));
            next.extend(page.canonical.clone());
            for u in next {
                push(&u, &mut queued, &mut frontier);
            }
            site.content_types.insert(final_url.clone(), resp.content_type.clone());
            site.pages.insert(final_url, page);
        }
    }
    Ok(site)
}

impl Site {
    /// Final status of a URL, if Kensa fetched it.
    pub fn status(&self, url: &str) -> Option<u16> {
        self.resolutions.get(url).map(|r| r.status)
    }

    /// Path of a URL relative to the site origin, for reports.
    pub fn path(&self, url: &str) -> String {
        url.strip_prefix(&self.origin)
            .map(|p| if p.is_empty() { "/".to_string() } else { p.to_string() })
            .unwrap_or_else(|| url.to_string())
    }
}
