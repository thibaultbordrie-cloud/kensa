//! The trap site: Lune & Lin, a fictional linen and tea shop in French, Japanese and Traditional
//! Chinese, with defects planted on purpose and an answer key that lists them.
//! A seed decides where each defect lands, so a tool cannot learn one fixed site by heart.

use std::fmt::Write as _;
use std::path::Path;

use crate::lang::to_simplified;
use crate::score::{AnswerKey, Expected};
use crate::serve::FIXTURE_ORIGIN as O;

// ---------------------------------------------------------------- content

/// Index 0 = French, 1 = Japanese, 2 = Traditional Chinese (Taiwan).
const DIRS: [&str; 3] = ["fr", "ja", "zh-tw"];
const HTML_LANG: [&str; 3] = ["fr", "ja", "zh-Hant-TW"];
const HREFLANG: [&str; 3] = ["fr", "ja", "zh-TW"];
const BRAND: [&str; 3] = ["Lune & Lin", "リュヌ・エ・ラン", "月與亞麻"];

struct Category {
    slug: &'static str,
    name: [&'static str; 3],
}

const CATEGORIES: [Category; 3] = [
    Category { slug: "papeterie", name: ["Papeterie", "紙と文具", "文具與紙品"] },
    Category { slug: "the", name: ["Thé", "お茶", "茶品"] },
    Category { slug: "maison", name: ["Maison", "暮らしの道具", "居家生活"] },
];

struct Product {
    slug: &'static str,
    category: usize,
    name: [&'static str; 3],
    /// Shown price, then the JSON-LD price, per market.
    price: [(&'static str, &'static str); 3],
}

const PRODUCTS: [Product; 6] = [
    Product {
        slug: "carnet-a5",
        category: 0,
        name: ["Carnet A5 en lin lavé", "麻表紙のA5ノート", "亞麻封面A5筆記本"],
        price: [("29,90 €", "29.90"), ("¥4,980", "4980"), ("NT$1,090", "1090")],
    },
    Product {
        slug: "papier-lettre",
        category: 0,
        name: ["Papier à lettres vergé", "透かし入りの便箋セット", "直紋信紙組"],
        price: [("14,90 €", "14.90"), ("¥2,480", "2480"), ("NT$520", "520")],
    },
    Product {
        slug: "coffret-the",
        category: 1,
        name: ["Coffret de thés de Provence", "プロヴァンスの紅茶ギフト", "普羅旺斯茶禮盒"],
        price: [("34,90 €", "34.90"), ("¥5,980", "5980"), ("NT$1,290", "1290")],
    },
    Product {
        slug: "theiere",
        category: 1,
        name: ["Théière en fonte émaillée", "ホーロー鋳鉄のティーポット", "琺瑯鑄鐵茶壺"],
        price: [("59,90 €", "59.90"), ("¥9,980", "9980"), ("NT$2,190", "2190")],
    },
    Product {
        slug: "nappe",
        category: 2,
        name: ["Nappe en lin naturel", "天然リネンのテーブルクロス", "天然亞麻桌巾"],
        price: [("79,90 €", "79.90"), ("¥12,980", "12980"), ("NT$2,790", "2790")],
    },
    Product {
        slug: "torchon",
        category: 2,
        name: ["Torchons en lin, lot de 2", "リネンのキッチンクロス2枚組", "亞麻廚房布兩件組"],
        price: [("19,90 €", "19.90"), ("¥2,980", "2980"), ("NT$590", "590")],
    },
];

const CURRENCY: [&str; 3] = ["EUR", "JPY", "TWD"];

const NAV_HOME: [&str; 3] = ["Accueil", "ホーム", "首頁"];
const NAV_ABOUT: [&str; 3] = ["À propos", "私たちについて", "關於我們"];
const NAV_GUIDE: [&str; 3] = ["Le guide", "読みもの", "選購指南"];
const NAV_CART: [&str; 3] = ["Panier", "カート", "購物車"];
const LANG_NAME: [&str; 3] = ["Français", "日本語", "繁體中文"];
const FOOTER: [&str; 3] = [
    "Lune & Lin est une marque fictive, créée pour tester des outils SEO.",
    "リュヌ・エ・ランは、SEOツールの検証のために作られた架空のブランドです。",
    "月與亞麻是為了測試SEO工具而設計的虛構品牌。",
];

fn home_intro(l: usize) -> &'static str {
    [
        "Lune & Lin crée de la papeterie, des thés et des objets du quotidien en lin, dessinés en France et fabriqués dans de petits ateliers. Nous livrons en France, au Japon et à Taïwan.",
        "リュヌ・エ・ランは、フランスでデザインし、小さな工房で仕上げたリネンの文具、お茶、暮らしの道具をお届けしています。日本へは東京の倉庫から発送します。",
        "月與亞麻在法國設計亞麻文具、茶品與居家用品，並交由小型工坊製作。我們從台北的倉庫出貨，讓台灣的顧客也能輕鬆選購。",
    ][l]
}

fn category_intro(l: usize, c: usize) -> String {
    let name = CATEGORIES[c].name[l];
    match l {
        0 => format!("{name} : une petite sélection de pièces simples et durables, choisies pour être utilisées tous les jours et offertes sans hésiter."),
        1 => format!("{name}のページです。毎日使えて、贈りものにもなる、シンプルで長く使えるものだけを選びました。"),
        _ => format!("{name}：我們只挑選簡單、耐用、每天都會用到的商品，也很適合當作禮物送給親友。"),
    }
}

fn product_copy(l: usize, p: &Product) -> String {
    let name = p.name[l];
    match l {
        0 => format!("{name}, fabriqué en France à partir de lin cultivé en Normandie et lavé à la pierre pour un toucher doux dès le premier jour. Livraison offerte dès 60 € en France métropolitaine."),
        1 => format!("{name}。ノルマンディー産のリネンをフランスの工房で仕上げました。最初からやわらかな手ざわりです。8,000円以上のご注文で送料無料です。"),
        _ => format!("{name}，以諾曼第種植的亞麻在法國製作，經過石洗處理，第一天使用就很柔軟。台灣訂單滿NT$1,800免運費，我們會用心為您包裝。"),
    }
}

fn product_meta(l: usize, p: &Product) -> String {
    let name = p.name[l];
    match l {
        0 => format!("{name} par Lune & Lin : lin de Normandie, fabrication française, livraison offerte dès 60 €."),
        1 => format!("{name}。フランスの工房で仕上げたリネン製品です。8,000円以上で送料無料。"),
        _ => format!("{name}，法國工坊製作的亞麻商品。台灣滿NT$1,800免運費。"),
    }
}

const ABOUT: [&str; 3] = [
    "Lune & Lin est née d'une idée simple : des objets en lin qui durent, et des thés qui donnent envie d'écrire une lettre. Nos ateliers partenaires sont en Normandie, dans les Vosges et en Provence. Nous expédions depuis Lyon, Tokyo et Taipei pour des délais courts.",
    "リュヌ・エ・ランは、長く使えるリネンの道具と、手紙を書きたくなるお茶をつくりたいという思いから始まりました。協力工房はノルマンディー、ヴォージュ、プロヴァンスにあります。リヨン、東京、台北から発送しています。",
    "月與亞麻的起點很簡單：做出耐用的亞麻用品，以及讓人想寫一封信的好茶。我們的合作工坊位於諾曼第、孚日與普羅旺斯，並從里昂、東京與台北出貨，讓您更快收到商品。",
];

const GUIDE_TITLE: [&str; 3] = ["Comment choisir son carnet en lin", "リネンのノートの選び方", "如何挑選亞麻筆記本"];

const GUIDE: [&str; 3] = [
    "Un bon carnet se choisit d'abord à la main. Le lin lavé donne une couverture souple qui se patine avec le temps, alors qu'une toile enduite reste nette plus longtemps. Regardez ensuite le papier : un grammage de 90 à 100 grammes accepte l'encre de stylo plume sans traverser la page. La reliure compte aussi. Une reliure cousue s'ouvre à plat et supporte des années d'usage, une reliure collée est plus légère mais fatigue plus vite. Enfin, pensez au format : le A5 suit partout, le A4 laisse de la place aux croquis et aux plans. Prenez le temps de comparer, de toucher et de feuilleter avant de décider.",
    "よいノートは、まず手に取って選びます。洗いをかけたリネンの表紙はやわらかく、使うほどに風合いが増します。コーティングした布はきれいな状態が長く続きます。次に紙を見てください。90から100グラムの紙なら、万年筆のインクも裏に抜けにくいです。とじ方も大切です。糸かがりのノートは平らに開き、何年も使えます。のりでとじたノートは軽いですが、早く傷みます。最後にサイズです。A5は持ち歩きやすく、A4はスケッチや計画にゆとりがあります。比べて、さわって、めくってから決めましょう。",
    "挑選一本好筆記本，先用手感受。水洗亞麻封面柔軟，越用越有味道；上膠布面則能長時間保持整潔。接著看紙張：90到100克的紙，用鋼筆書寫也不容易透到背面。裝訂方式也很重要。穿線裝訂可以完全攤平，用上好幾年也沒問題；膠裝比較輕，但比較容易損壞。最後是尺寸：A5方便隨身攜帶，A4則有更多空間畫草圖與寫計畫。多比較、多觸摸、多翻閱，再做決定。",
];

// ---------------------------------------------------------------- site model

/// Pages that exist in the three languages. 0 home, 1-3 categories, 4-9 products, 10 about, 11 guide.
const GROUPS: usize = 12;
const GUIDE_GROUP: usize = 11;

fn group_path(g: usize, l: usize) -> String {
    let d = DIRS[l];
    match g {
        0 => format!("/{d}/"),
        1..=3 => format!("/{d}/categories/{}.html", CATEGORIES[g - 1].slug),
        4..=9 => format!("/{d}/produits/{}.html", PRODUCTS[g - 4].slug),
        10 => format!("/{d}/a-propos.html"),
        _ => format!("/{d}/guide-carnet.html"),
    }
}

/// What is wrong with one page, decided by the seed.
#[derive(Default, Clone)]
struct Tweaks {
    drop_codes: Vec<&'static str>,
    rename_code: Option<(&'static str, &'static str)>,
    retarget: Option<(&'static str, String)>,
    simplified: bool,
    untranslated: bool,
    long_title: bool,
    canonical_to: Option<String>,
    title: Option<String>,
    no_description: bool,
    no_h1: bool,
    img_no_alt: bool,
    broken_link: bool,
    chain_link: bool,
    jsonld_invalid: bool,
    jsonld_no_currency: bool,
    big5: bool,
}

/// splitmix64: tiny, deterministic, no dependency.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
    fn pick<T: Copy>(&mut self, items: &[T]) -> T {
        items[self.below(items.len())]
    }
    fn shuffle<T>(&mut self, items: &mut [T]) {
        for i in (1..items.len()).rev() {
            items.swap(i, self.below(i + 1));
        }
    }
}

pub struct Plan {
    tweaks: Vec<[Tweaks; 3]>,
    no_x_default_group: usize,
    cart_in_sitemap: usize,
    ai_block: (&'static str, &'static str),
    pub key: AnswerKey,
}

fn expect(key: &mut Vec<Expected>, rule: &str, url: String, why: &str) {
    key.push(Expected { rule: rule.into(), url, why: why.into() });
}

pub fn plan(seed: u64) -> Plan {
    let mut rng = Rng(seed);
    let mut t: Vec<[Tweaks; 3]> = (0..GROUPS).map(|_| Default::default()).collect();
    let mut key = Vec::new();

    // Defects that involve several pages of one group get a group each.
    let canonical_group = 4 + rng.below(6);
    let mut free: Vec<usize> = (0..GROUP_GUIDE_EXCLUDED).filter(|g| *g != canonical_group).collect();
    rng.shuffle(&mut free);
    let simplified_group = *free.iter().find(|g| **g != 0).expect("groups");
    free.retain(|g| *g != simplified_group);
    // Without x-default on the home pages, the language chooser at / would lose its return links.
    let no_default = *free.iter().find(|g| **g != 0).expect("groups");
    free.retain(|g| *g != no_default);
    let [jp, no_return, no_self, typo, zh_cn] = [free[0], free[1], free[2], free[3], free[4]];

    t[canonical_group][1].canonical_to = Some(group_path(canonical_group, 0));
    expect(
        &mut key,
        "canonical-other-language",
        group_path(canonical_group, 1),
        "the Japanese product declares the French product as canonical",
    );

    t[simplified_group][2].simplified = true;
    expect(
        &mut key,
        "zh-script-mismatch",
        group_path(simplified_group, 2),
        "declared zh-Hant-TW, written in Simplified Chinese",
    );

    t[jp][0].rename_code = Some(("ja", "jp"));
    expect(&mut key, "hreflang-invalid-code", group_path(jp, 0), "hreflang=\"jp\" instead of \"ja\"");

    t[no_return][1].drop_codes = vec!["fr", "x-default"];
    expect(
        &mut key,
        "hreflang-no-return",
        group_path(no_return, 1),
        "the Japanese page does not name the French page back",
    );

    t[no_self][2].drop_codes = vec!["zh-TW"];
    expect(&mut key, "hreflang-no-self", group_path(no_self, 2), "the Chinese page omits itself");

    let typo_url = group_path(typo, 2).replace(".html", "-.html").replace("/zh-tw/", "/zh-tw/-/");
    t[typo][0].retarget = Some(("zh-TW", typo_url));
    expect(&mut key, "hreflang-target-error", group_path(typo, 0), "the zh-TW alternate has a typo and answers 404");
    expect(&mut key, "hreflang-no-return", group_path(typo, 0), "so the real Chinese page is not named back");

    t[zh_cn][1].rename_code = Some(("zh-TW", "zh-CN"));
    expect(&mut key, "hreflang-lang-mismatch", group_path(zh_cn, 1), "zh-CN points to a Traditional Chinese page");

    t[GUIDE_GROUP][1].untranslated = true;
    expect(
        &mut key,
        "html-lang-mismatch",
        group_path(GUIDE_GROUP, 1),
        "lang=\"ja\" on the untranslated French article",
    );

    let wide = 4 + rng.below(6);
    t[wide][1].long_title = true;
    expect(
        &mut key,
        "title-too-wide",
        group_path(wide, 1),
        "a long Japanese title passes 100 columns: CJK characters count double",
    );

    let mut cats = vec![1, 2, 3];
    rng.shuffle(&mut cats);
    t[cats[1]][0].title = Some(page_title(cats[0], 0));
    expect(&mut key, "title-duplicate", group_path(cats[0], 0), "two French categories share one title");
    expect(&mut key, "title-duplicate", group_path(cats[1], 0), "two French categories share one title");

    let (g, l) = (rng.below(GROUPS), rng.below(3));
    t[g][l].no_description = true;
    expect(&mut key, "description-missing", group_path(g, l), "no meta description");

    let (g, l) = (rng.pick(&[0, 1, 2, 3, 10]), rng.below(3));
    t[g][l].no_h1 = true;
    expect(&mut key, "h1-missing", group_path(g, l), "no h1");

    let (g, l) = (4 + rng.below(6), rng.below(3));
    t[g][l].img_no_alt = true;
    expect(&mut key, "img-alt-missing", group_path(g, l), "the product photo has no alt");

    let g = rng.below(GROUPS);
    t[g][2].broken_link = true;
    expect(&mut key, "link-broken", group_path(g, 2), "links to a product page that does not exist");

    let g = rng.below(GROUPS);
    t[g][0].chain_link = true;
    expect(&mut key, "redirect-chain", group_path(g, 0), "links to an old URL that redirects twice");

    let mut prods = vec![4, 5, 6, 7, 8, 9];
    rng.shuffle(&mut prods);
    let (l1, l2) = (rng.below(3), rng.below(3));
    t[prods[0]][l1].jsonld_invalid = true;
    expect(&mut key, "jsonld-invalid", group_path(prods[0], l1), "a trailing comma breaks the JSON-LD");
    t[prods[1]][l2].jsonld_no_currency = true;
    expect(&mut key, "jsonld-product-incomplete", group_path(prods[1], l2), "the offer has no priceCurrency");

    let g = rng.below(GROUPS);
    t[g][2].big5 = true;
    expect(&mut key, "charset-not-utf8", group_path(g, 2), "meta charset says Big5");

    let cart = rng.below(3);
    expect(
        &mut key,
        "sitemap-url-noindex",
        format!("/{}/panier.html", DIRS[cart]),
        "the noindex cart is in the sitemap",
    );
    expect(
        &mut key,
        "sitemap-url-error",
        "/fr/produits/ancien-modele.html".into(),
        "a removed product is still in the sitemap",
    );

    let ai_block =
        (rng.pick(&["OAI-SearchBot", "Claude-SearchBot", "PerplexityBot"]), rng.pick(&["/", "/ja/", "/zh-tw/"]));
    expect(&mut key, "ai-search-blocked", "/robots.txt".into(), "robots.txt blocks an AI search crawler");
    expect(&mut key, "llms-txt-invalid", "/llms.txt".into(), "llms.txt links to a page that does not exist");
    expect(
        &mut key,
        "content-js-only",
        "/ja/line-campaign.html".into(),
        "the LINE campaign page is drawn by JavaScript",
    );

    key.sort();
    Plan {
        tweaks: t,
        no_x_default_group: no_default,
        cart_in_sitemap: cart,
        ai_block,
        key: AnswerKey { seed, origin: O.into(), expected: key },
    }
}

/// Groups that can carry a cross-page defect (the guide is kept for the untranslated page).
const GROUP_GUIDE_EXCLUDED: usize = GUIDE_GROUP;

// ---------------------------------------------------------------- rendering

fn page_title(g: usize, l: usize) -> String {
    let sep = if l == 0 { " · " } else { " | " };
    let brand = BRAND[l];
    match g {
        0 => format!("{brand}{sep}{}", ["Papeterie, thé et lin", "リネンの文具とお茶", "亞麻文具與茶品"][l]),
        1..=3 => format!("{}{sep}{brand}", CATEGORIES[g - 1].name[l]),
        4..=9 => format!("{}{sep}{brand}", PRODUCTS[g - 4].name[l]),
        10 => format!("{}{sep}{brand}", NAV_ABOUT[l]),
        _ => format!("{}{sep}{brand}", GUIDE_TITLE[l]),
    }
}

fn page_description(g: usize, l: usize) -> String {
    match g {
        0 => home_intro(l).split(['.', '。']).next().unwrap_or("").to_string() + [".", "。", "。"][l],
        1..=3 => category_intro(l, g - 1),
        4..=9 => product_meta(l, &PRODUCTS[g - 4]),
        10 => [
            "L'histoire de Lune & Lin et de ses ateliers partenaires.",
            "リュヌ・エ・ランと協力工房の物語。",
            "月與亞麻與合作工坊的故事。",
        ][l]
            .into(),
        _ => [
            "Couverture, papier, reliure et format : nos conseils pour bien choisir.",
            "表紙、紙、とじ方、サイズ。選び方のポイントをまとめました。",
            "封面、紙張、裝訂與尺寸，挑選筆記本的重點整理。",
        ][l]
            .into(),
    }
}

fn url(path: &str) -> String {
    format!("{O}{path}")
}

struct Head<'a> {
    lang: &'a str,
    title: Option<String>,
    description: Option<String>,
    canonical: String,
    alternates: Vec<(String, String)>,
    charset: &'a str,
    jsonld: Vec<String>,
    noindex: bool,
}

fn head(h: &Head) -> String {
    let mut s = format!("<!doctype html>\n<html lang=\"{}\">\n<head>\n<meta charset=\"{}\">\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n", h.lang, h.charset);
    if let Some(t) = &h.title {
        let _ = writeln!(s, "<title>{}</title>", esc(t));
    }
    if let Some(d) = &h.description {
        let _ = writeln!(s, "<meta name=\"description\" content=\"{}\">", esc(d));
    }
    if h.noindex {
        s.push_str("<meta name=\"robots\" content=\"noindex\">\n");
    }
    let _ = writeln!(s, "<link rel=\"canonical\" href=\"{}\">", h.canonical);
    for (code, href) in &h.alternates {
        let _ = writeln!(s, "<link rel=\"alternate\" hreflang=\"{code}\" href=\"{href}\">");
    }
    for j in &h.jsonld {
        let _ = writeln!(s, "<script type=\"application/ld+json\">{j}</script>");
    }
    s.push_str("</head>\n");
    s
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('"', "&quot;").replace('<', "&lt;")
}

fn nav(l: usize, g: Option<usize>) -> String {
    let d = DIRS[l];
    let mut s = format!("<header><a href=\"/{d}/\">{}</a>\n<nav>", esc(BRAND[l]));
    let _ = write!(s, "<a href=\"/{d}/\">{}</a> ", NAV_HOME[l]);
    for c in &CATEGORIES {
        let _ = write!(s, "<a href=\"/{d}/categories/{}.html\">{}</a> ", c.slug, c.name[l]);
    }
    let _ = write!(s, "<a href=\"/{d}/a-propos.html\">{}</a> <a href=\"/{d}/guide-carnet.html\">{}</a> <a href=\"/{d}/panier.html\">{}</a></nav>\n<nav>",
        NAV_ABOUT[l], NAV_GUIDE[l], NAV_CART[l]);
    for other in 0..3 {
        if other != l {
            let target = g.map(|g| group_path(g, other)).unwrap_or(format!("/{}/", DIRS[other]));
            let _ = write!(s, "<a href=\"{target}\" hreflang=\"{}\">{}</a> ", HREFLANG[other], LANG_NAME[other]);
        }
    }
    s.push_str("</nav></header>\n");
    s
}

fn footer(l: usize) -> String {
    format!("<footer><p>{}</p></footer>\n</body>\n</html>\n", FOOTER[l])
}

fn organization() -> String {
    format!(
        r#"{{"@context":"https://schema.org","@type":"Organization","name":"Lune & Lin","url":"{O}/","description":"Fictional brand of linen stationery, tea and home goods."}}"#
    )
}

fn product_jsonld(p: &Product, l: usize, tw: &Tweaks) -> String {
    let currency = if tw.jsonld_no_currency { String::new() } else { format!(r#","priceCurrency":"{}""#, CURRENCY[l]) };
    let trailing = if tw.jsonld_invalid { "," } else { "" };
    format!(
        r#"{{"@context":"https://schema.org","@type":"Product","name":"{}","sku":"LL-{}","offers":{{"@type":"Offer","price":"{}"{currency},"availability":"https://schema.org/InStock"{trailing}}}}}"#,
        p.name[l],
        p.slug.to_uppercase(),
        p.price[l].1
    )
}

fn group_page(g: usize, l: usize, plan: &Plan) -> String {
    let tw = &plan.tweaks[g][l];
    let self_url = url(&group_path(g, l));
    let mut alternates: Vec<(String, String)> =
        (0..3).map(|o| (HREFLANG[o].to_string(), url(&group_path(g, o)))).collect();
    if g != plan.no_x_default_group {
        alternates.push(("x-default".into(), if g == 0 { url("/") } else { url(&group_path(g, 0)) }));
    }
    alternates.retain(|(c, _)| !tw.drop_codes.contains(&c.as_str()));
    for (code, href) in alternates.iter_mut() {
        if let Some((from, to)) = tw.rename_code {
            if code == from {
                *code = to.to_string();
            }
        }
        if let Some((which, target)) = &tw.retarget {
            if code == which {
                *href = url(target);
            }
        }
    }
    let title = if tw.long_title {
        let name = PRODUCTS[g - 4].name[l];
        format!("{name}　フランスの工房で丁寧に仕上げた天然素材の上質なアイテムを毎日の暮らしに | {}", BRAND[l])
    } else {
        tw.title.clone().unwrap_or_else(|| page_title(g, l))
    };
    let mut jsonld = Vec::new();
    if g == 0 {
        jsonld.push(organization());
    }
    if (4..=9).contains(&g) {
        jsonld.push(product_jsonld(&PRODUCTS[g - 4], l, tw));
    }
    let h = Head {
        lang: HTML_LANG[l],
        title: Some(title),
        description: (!tw.no_description).then(|| page_description(g, l)),
        canonical: tw.canonical_to.as_deref().map(url).unwrap_or(self_url),
        alternates,
        charset: if tw.big5 { "big5" } else { "utf-8" },
        jsonld,
        noindex: false,
    };
    let d = DIRS[l];
    let text_lang = if tw.untranslated { 0 } else { l };
    let mut main = String::from("<main>\n");
    let h1 = |s: &str| if tw.no_h1 { format!("<p class=\"lead\">{s}</p>\n") } else { format!("<h1>{s}</h1>\n") };
    match g {
        0 => {
            main += &h1(&format!(
                "{} · {}",
                BRAND[l],
                ["Papeterie, thé et lin", "リネンの文具とお茶", "亞麻文具與茶品"][l]
            ));
            let _ = writeln!(main, "<p>{}</p>\n<ul>", home_intro(l));
            for p in &PRODUCTS {
                let _ = writeln!(
                    main,
                    "<li><a href=\"/{d}/produits/{}.html\">{}</a> {}</li>",
                    p.slug, p.name[l], p.price[l].0
                );
            }
            main += "</ul>\n";
            if l == 1 {
                main += "<p><a href=\"/ja/line-campaign.html\">LINE友だち限定キャンペーン</a></p>\n";
            }
        }
        1..=3 => {
            let c = g - 1;
            main += &h1(CATEGORIES[c].name[l]);
            let _ = writeln!(main, "<p>{}</p>\n<ul>", category_intro(l, c));
            for p in PRODUCTS.iter().filter(|p| p.category == c) {
                let _ = writeln!(
                    main,
                    "<li><a href=\"/{d}/produits/{}.html\">{}</a> {}</li>",
                    p.slug, p.name[l], p.price[l].0
                );
            }
            main += "</ul>\n";
        }
        4..=9 => {
            let p = &PRODUCTS[g - 4];
            main += &h1(p.name[l]);
            let alt = if tw.img_no_alt { String::new() } else { format!(" alt=\"{}\"", esc(p.name[l])) };
            let _ = writeln!(main, "<img src=\"/img/{}.svg\"{alt} width=\"600\" height=\"400\">", p.slug);
            let _ = writeln!(main, "<p>{}</p>\n<p class=\"price\">{}</p>", product_copy(l, p), p.price[l].0);
            let _ = writeln!(
                main,
                "<p><a href=\"/{d}/categories/{}.html\">{}</a></p>",
                CATEGORIES[p.category].slug, CATEGORIES[p.category].name[l]
            );
        }
        10 => {
            main += &h1(NAV_ABOUT[l]);
            let _ = writeln!(main, "<p>{}</p>", ABOUT[l]);
        }
        _ => {
            main += &h1(GUIDE_TITLE[text_lang]);
            let _ = writeln!(main, "<p>{}</p>", GUIDE[text_lang]);
        }
    }
    if tw.broken_link {
        main += "<p><a href=\"/zh-tw/produits/cadeau.html\">禮物卡</a></p>\n";
    }
    if tw.chain_link {
        main += "<p><a href=\"/fr/ancien-catalogue.html\">Ancien catalogue</a></p>\n";
    }
    main += "</main>\n";
    let mut body = format!("<body>\n{}{main}", nav(l, Some(g)));
    if tw.simplified {
        body = to_simplified(&body);
    }
    head(&h) + &body + &footer(l)
}

fn root_page() -> String {
    let mut alternates: Vec<(String, String)> =
        (0..3).map(|l| (HREFLANG[l].to_string(), url(&group_path(0, l)))).collect();
    alternates.push(("x-default".into(), url("/")));
    let h = Head {
        lang: "en",
        title: Some("Lune & Lin · Choose your shop".into()),
        description: Some(
            "Lune & Lin, a fictional brand of linen stationery, tea and home goods, sold in France, Japan and Taiwan."
                .into(),
        ),
        canonical: url("/"),
        alternates,
        charset: "utf-8",
        jsonld: vec![organization()],
        noindex: false,
    };
    let mut body = String::from("<body>\n<main>\n<h1>Lune &amp; Lin</h1>\n<p>Linen stationery, tea and home goods designed in France. This brand is fictional: the site exists to test SEO tools. Choose your shop below.</p>\n<ul>\n");
    for l in 0..3 {
        let _ = writeln!(body, "<li><a href=\"/{}/\" hreflang=\"{}\">{}</a></li>", DIRS[l], HREFLANG[l], LANG_NAME[l]);
    }
    body += "</ul>\n</main>\n</body>\n</html>\n";
    head(&h) + &body
}

fn cart_page(l: usize) -> String {
    let h = Head {
        lang: HTML_LANG[l],
        title: Some(format!("{} | {}", NAV_CART[l], BRAND[l])),
        description: None,
        canonical: url(&format!("/{}/panier.html", DIRS[l])),
        alternates: vec![],
        charset: "utf-8",
        jsonld: vec![],
        noindex: true,
    };
    let empty = ["Votre panier est vide.", "カートに商品はありません。", "您的購物車是空的。"][l];
    head(&h)
        + &format!("<body>\n{}<main>\n<h1>{}</h1>\n<p>{empty}</p>\n</main>\n", nav(l, None), NAV_CART[l])
        + &footer(l)
}

fn js_page() -> String {
    let h = Head {
        lang: "ja",
        title: Some("LINE友だち限定キャンペーン | リュヌ・エ・ラン".into()),
        description: Some("LINEで友だち追加すると、リネンのノートが10%オフになるキャンペーンです。".into()),
        canonical: url("/ja/line-campaign.html"),
        alternates: vec![],
        charset: "utf-8",
        jsonld: vec![],
        noindex: false,
    };
    head(&h) + "<body>\n<div id=\"app\"></div>\n<script>\ndocument.getElementById('app').innerHTML = '<h1>LINE友だち限定キャンペーン</h1><p>友だち追加でリネンのノートが10%オフ。</p>';\n</script>\n</body>\n</html>\n"
}

fn robots(plan: &Plan) -> String {
    let (bot, path) = plan.ai_block;
    format!("# Lune & Lin (fictional)\nUser-agent: *\nAllow: /\n\n# We opt out of AI training.\nUser-agent: GPTBot\nUser-agent: CCBot\nDisallow: /\n\nUser-agent: {bot}\nDisallow: {path}\n\nSitemap: {O}/sitemap.xml\n")
}

fn llms() -> String {
    let mut s = String::from("# Lune & Lin\n\n> Linen stationery, tea and home goods designed in France, sold in France, Japan and Taiwan. A fictional brand made to test SEO tools.\n\n## Shops\n\n");
    for l in 0..3 {
        let _ = writeln!(s, "- [{}]({O}/{}/): {}", LANG_NAME[l], DIRS[l], BRAND[l]);
    }
    s += "\n## Guides\n\n";
    let _ = writeln!(s, "- [{}]({O}/fr/guide-carnet.html)", GUIDE_TITLE[0]);
    let _ = writeln!(s, "- [Livraison et retours]({O}/fr/livraison.html)");
    s
}

fn sitemap(plan: &Plan) -> String {
    let mut urls = vec!["/".to_string()];
    for g in 0..GROUPS {
        for l in 0..3 {
            urls.push(group_path(g, l));
        }
    }
    urls.push("/ja/line-campaign.html".into());
    urls.push(format!("/{}/panier.html", DIRS[plan.cart_in_sitemap]));
    urls.push("/fr/produits/ancien-modele.html".into());
    let mut s = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n",
    );
    for u in urls {
        let _ = writeln!(s, "  <url><loc>{}</loc></url>", url(&u));
    }
    s + "</urlset>\n"
}

fn image(label: &str) -> String {
    format!("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"600\" height=\"400\"><rect width=\"600\" height=\"400\" fill=\"#e9e4da\"/><text x=\"300\" y=\"210\" font-size=\"28\" text-anchor=\"middle\" fill=\"#6b5e4b\">{label}</text></svg>\n")
}

/// Write the site and its answer key (`answer-key.json`, outside the served folder).
pub fn write(dir: &Path, seed: u64) -> anyhow::Result<AnswerKey> {
    let plan = plan(seed);
    let site = dir.join("site");
    let mut files: Vec<(String, String)> = vec![
        ("index.html".into(), root_page()),
        ("robots.txt".into(), robots(&plan)),
        ("llms.txt".into(), llms()),
        ("sitemap.xml".into(), sitemap(&plan)),
        ("ja/line-campaign.html".into(), js_page()),
        (
            "_redirects".into(),
            "/fr/ancien-catalogue.html /fr/catalogue.html 301\n/fr/catalogue.html /fr/categories/papeterie.html 301\n"
                .into(),
        ),
    ];
    for g in 0..GROUPS {
        for l in 0..3 {
            let mut path = group_path(g, l).trim_start_matches('/').to_string();
            if path.ends_with('/') {
                path.push_str("index.html");
            }
            files.push((path, group_page(g, l, &plan)));
        }
    }
    for (l, dir) in DIRS.iter().enumerate() {
        files.push((format!("{dir}/panier.html"), cart_page(l)));
    }
    for p in &PRODUCTS {
        files.push((format!("img/{}.svg", p.slug), image(p.name[0])));
    }
    if site.exists() {
        std::fs::remove_dir_all(&site)?;
    }
    for (path, content) in files {
        let target = site.join(path);
        std::fs::create_dir_all(target.parent().expect("has parent"))?;
        std::fs::write(target, content)?;
    }
    std::fs::write(dir.join("answer-key.json"), serde_json::to_string_pretty(&plan.key)? + "\n")?;
    Ok(plan.key)
}
