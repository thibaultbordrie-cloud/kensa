# Kensa 検査

[![ci](https://github.com/thibaultbordrie-cloud/kensa/actions/workflows/ci.yml/badge.svg)](https://github.com/thibaultbordrie-cloud/kensa/actions/workflows/ci.yml)
![Rust](https://img.shields.io/badge/Rust-single%20binary-B7410E?logo=rust&logoColor=white)
![Benchmark](https://img.shields.io/badge/trap%20site-F1%201.00%20on%20100%20seeds-2E7D32)
![License: MIT](https://img.shields.io/badge/License-MIT-lightgrey)

**An SEO auditor for sites in French, Japanese and Chinese, built for search engines and for AI assistants. It ships with a trap site and its answer key, so every claim it makes can be measured.**

*Kensa* means "inspection". One binary, no runtime to install, about 2 seconds to crawl and score a 41-page site 100 times.

[日本語](#日本語での概要) · [繁體中文](#中文簡介繁體) · [Français](#en-bref-français)

## Why

Most SEO tools are written for one English site. Sites that sell in France, Japan and Taiwan break in other places:

- **hreflang**: `jp` instead of `ja`, `en-UK` instead of `en-GB`, a missing return link, an alternate with a typo that answers 404.
- **Chinese scripts**: a page declared `zh-Hant-TW` but written in Simplified characters, or a Traditional page announced as `zh-CN`.
- **Title width**: Google cuts titles by width, and a CJK character is twice as wide as a Latin one. A tool that counts characters lets a 58-character Japanese title through; Kensa measures 112 columns.
- **Forgotten translations**: `lang="ja"` on a page whose article is still in French.
- **AI search (GEO)**: robots.txt that blocks the crawlers AI assistants use to cite pages (OAI-SearchBot, Claude-SearchBot, PerplexityBot), pages drawn only by JavaScript that those crawlers cannot read, a broken `llms.txt`, invalid JSON-LD.

Kensa tells **search crawlers** apart from **training crawlers**: blocking GPTBot or Google-Extended is a legitimate choice (a notice); blocking OAI-SearchBot removes the site from ChatGPT search answers (an error).

## Quick start

```bash
cargo install --git https://github.com/thibaultbordrie-cloud/kensa   # or download a binary from Releases
kensa bench                                    # build the trap site, serve it, audit it, score it
kensa audit https://www.example.com/ --max-pages 200
kensa audit https://www.example.com/ --format markdown --out report.md
kensa rules                                    # the 38 rules
```

The trap site, step by step:

```bash
kensa fixture --out lab --seed 7               # lab/site + lab/answer-key.json
kensa serve lab/site --port 8080               # in a second terminal
kensa audit http://127.0.0.1:8080/ --format json --out lab/report.json
kensa score lab/report.json lab/answer-key.json
```

## What a report looks like

```text
Kensa 検査 · http://127.0.0.1:8099 · 41 pages
12 errors · 13 warnings · 5 notices

ERROR   hreflang-invalid-code        /fr/a-propos.html
        "jp" is not valid: use "ja"
ERROR   hreflang-target-error        /fr/produits/nappe.html
        hreflang="zh-TW" → /zh-tw/-/produits/nappe-.html answers 404
ERROR   html-lang-mismatch           /ja/guide-carnet.html
        lang="ja" but the text looks Latin-script (latin 510, kana 48, han 18)
ERROR   content-js-only              /ja/line-campaign.html
        0 letters of text without JavaScript, 1 script(s)
ERROR   canonical-other-language     /ja/produits/theiere.html
        a ja page declares a fr page as canonical: /fr/produits/theiere.html
ERROR   ai-search-blocked            /robots.txt
        Claude-SearchBot (Anthropic): /zh-tw/
ERROR   zh-script-mismatch           /zh-tw/produits/torchon.html
        lang="zh-Hant-TW" but the text has 17 Simplified-only and 2 Traditional-only characters
WARNING hreflang-lang-mismatch       /ja/produits/papier-lettre.html
        hreflang="zh-CN" points to /zh-tw/produits/papier-lettre.html, a page in Traditional Chinese
WARNING title-too-wide               /ja/produits/papier-lettre.html
        112 columns (58 characters): "透かし入りの便箋セット フランスの工房で…"
notice  ai-training-blocked          /robots.txt
        GPTBot (OpenAI): /, /fr/, /ja/, /zh-tw/; CCBot (Common Crawl): /, /fr/, /ja/, /zh-tw/
…
```

## The trap site and its answer key

`kensa fixture` writes **Lune & Lin**, a fictional French shop of linen stationery and tea, in French, Japanese and Traditional Chinese (41 pages, robots.txt, sitemap, llms.txt, redirects). It plants **25 defects** across 23 rules and writes the list in `answer-key.json`. The **seed** decides which pages get which defect, so a tool cannot learn one fixed site by heart.

`kensa score` compares any JSON report with the key: precision (no false alarms), recall (nothing missed) and F1, overall and per rule. Notices are information and are not scored. The test suite runs 25 seeds on every push; Kensa finds every defect and raises no false alarm. Any other tool, script or AI agent can be scored the same way.

The fixture uses the reserved domain `lune-et-lin.example`; `kensa serve` maps it to its local address, so the benchmark needs no network.

## What it checks

38 rules in five groups, listed with their severity in [docs/RULES.md](docs/RULES.md):

| Group | Examples |
|---|---|
| hreflang | invalid code with the intended fix, no return link, no self link, target in error, duplicate code, code that contradicts the target's language or Chinese script, relative URL, no x-default |
| language | html lang missing, invalid or contradicted by the text, zh-Hant vs zh-Hans, non UTF-8 charset |
| content | title missing, duplicated or too wide (CJK counted double), full-width ＡＢＣ in titles, description, h1, image alt |
| indexing | canonical in error or pointing to another language, broken internal links, redirect chains, sitemap URLs in error or noindex |
| AI search | AI search crawlers blocked, training crawlers blocked (notice), JavaScript-only content, llms.txt missing or broken, invalid JSON-LD, incomplete Product, no Organization |

Language detection is deliberately simple and explainable: it counts Latin letters, kana, kanji/hanzi and hangul, and a short table of characters that differ between Traditional and Simplified Chinese. Every message shows the counts it relied on.

## Use it in CI

```yaml
- run: cargo install --git https://github.com/thibaultbordrie-cloud/kensa
- run: kensa audit https://staging.example.com/ --fail-on error --format markdown --out kensa.md
```

`--fail-on error` (or `warning`) makes the command exit with code 1 when such a finding exists.

## Design

- **Rust**, one binary for Linux, macOS and Windows. `reqwest` with rustls (system certificates are trusted too, which helps behind a company proxy), `scraper` (html5ever) for HTML.
- **Polite crawler**: respects robots.txt for its own user agent, 4 parallel requests by default, same origin only, redirects followed by hand so chains can be reported.
- **RFC 9309 robots.txt matching**: grouped user agents, longest match wins, `Allow` wins ties, `*` and `$` patterns.
- **No JavaScript rendering**, on purpose: Kensa reads pages the way AI crawlers do.
- **Tests**: unit tests per module, the 25-seed benchmark, a test that keeps docs/RULES.md in sync with the code, and an anonymity guard.

Limits today: hreflang declared in sitemaps or HTTP headers is not read yet, and the crawler does not check external links.

## Roadmap

1. SARIF output, to show findings in GitHub code scanning.
2. hreflang from sitemaps and `Link` headers.
3. Python bindings (PyO3), to call Kensa from notebooks and data pipelines.
4. Read-only MCP tool, so an AI agent can audit a site and be scored against the answer key.

## 日本語での概要

Kensa（検査）は、フランス語・日本語・中国語のサイトに特化したSEO監査ツールです。Rust製の単一バイナリで動きます。hreflang の誤り（`jp` と `ja` の取り違えなど）や、全角文字を2文字分として数えるタイトル幅をチェックします。翻訳し忘れたページ、繁体字と簡体字の取り違えも見つけます。AI検索向けの項目（AIクローラーのブロック、JavaScriptだけで描画されたページ、llms.txt、構造化データ）も確認します。架空のショップ「リュヌ・エ・ラン」のテストサイトと正解データが付属しています。適合率と再現率でツールの精度を測れます。

## 中文簡介（繁體）

Kensa（検査）是一套以 Rust 編寫的 SEO 檢查工具，專為法文、日文與中文網站設計。它會檢查 hreflang 錯誤，並以全形字元算兩格的方式計算標題寬度。它也能找出被標成 zh-Hant 卻用簡體字撰寫的頁面，以及被封鎖的 AI 搜尋爬蟲、只靠 JavaScript 顯示內容的頁面和結構化資料問題。工具附有虛構商店「月與亞麻」的測試網站與標準答案，可以用精確率與召回率衡量任何工具的表現。

## En bref (français)

Kensa est un auditeur SEO en Rust pour les sites en français, japonais et chinois, pensé aussi pour les moteurs de réponse IA (GEO). Il vérifie le hreflang, la largeur des titres en caractères CJK, le chinois traditionnel ou simplifié et les traductions oubliées. Il contrôle aussi les robots IA bloqués, les pages rendues en JavaScript, llms.txt et les données structurées. Il fournit un site piège avec son corrigé pour mesurer précision et rappel.

## License

MIT. Lune & Lin, its products and its texts are fictional.
