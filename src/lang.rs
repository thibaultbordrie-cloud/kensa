//! Language tags (hreflang, html lang) and script detection for French, Japanese and Chinese.

/// A parsed language tag: language, optional script, optional region.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tag {
    pub language: String,
    pub script: Option<String>,
    pub region: Option<String>,
}

/// Common mistakes seen on real sites, with the code that was meant.
const HINTS: &[(&str, &str)] = &[
    ("jp", "ja"),
    ("cn", "zh"),
    ("kr", "ko"),
    ("tw", "zh-TW"),
    ("hk", "zh-HK"),
    ("en-uk", "en-GB"),
    ("ja-jp-jp", "ja-JP"),
    ("zh-tc", "zh-Hant"),
    ("zh-sc", "zh-Hans"),
];

/// Parse a hreflang or html lang value. `x-default` is handled by the caller.
pub fn parse(code: &str) -> Result<Tag, String> {
    let lower = code.trim().to_ascii_lowercase().replace('_', "-");
    if let Some((_, good)) = HINTS.iter().find(|(bad, _)| *bad == lower) {
        return Err(format!("\"{code}\" is not valid: use \"{good}\""));
    }
    let parts: Vec<&str> = lower.split('-').collect();
    let language = parts[0];
    if language.len() != 2 || isolang::Language::from_639_1(language).is_none() {
        return Err(format!("\"{language}\" is not an ISO 639-1 language code"));
    }
    let mut tag = Tag { language: language.to_string(), script: None, region: None };
    for part in &parts[1..] {
        match part.len() {
            4 if tag.script.is_none() && tag.region.is_none() && part.chars().all(|c| c.is_ascii_alphabetic()) => {
                let mut s = part.to_string();
                s[..1].make_ascii_uppercase();
                tag.script = Some(s);
            }
            2 if tag.region.is_none() => {
                let up = part.to_ascii_uppercase();
                if up == "UK" {
                    return Err(format!("\"{code}\" uses UK: the ISO 3166 code is GB"));
                }
                if isocountry::CountryCode::for_alpha2(&up).is_err() {
                    return Err(format!("\"{up}\" is not an ISO 3166-1 country code"));
                }
                tag.region = Some(up);
            }
            _ => return Err(format!("\"{code}\" has an unexpected part \"{part}\"")),
        }
    }
    Ok(tag)
}

/// Which Chinese script a tag implies, if any.
pub fn zh_script(tag: &Tag) -> Option<ZhScript> {
    if tag.language != "zh" {
        return None;
    }
    match (tag.script.as_deref(), tag.region.as_deref()) {
        (Some("Hant"), _) => Some(ZhScript::Traditional),
        (Some("Hans"), _) => Some(ZhScript::Simplified),
        (None, Some("TW" | "HK" | "MO")) => Some(ZhScript::Traditional),
        (None, Some("CN" | "SG" | "MY")) => Some(ZhScript::Simplified),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZhScript {
    Traditional,
    Simplified,
}

/// Character counts by script in a piece of visible text.
#[derive(Debug, Default, Clone, Copy)]
pub struct Profile {
    pub latin: usize,
    pub kana: usize,
    pub han: usize,
    pub hangul: usize,
    pub traditional: usize,
    pub simplified: usize,
}

/// Pairs of (traditional, simplified) characters that differ and are frequent in shop copy.
const ZH_PAIRS: &str = "們们這这個个說说來来為为會会時时國国灣湾體体與与麼么點点開开關关東东車车書书長长門门問问間间電电買买賣卖價价錢钱貨货運运選选購购優优質质質质禮礼節节還还讓让現现發发們们親亲愛爱寫写筆笔紙纸題题頁页網网藝艺術术實实務务員员應应對对從从這这將将專专單单號号";

/// Convert the Traditional characters Kensa knows to Simplified (used to plant fixture defects).
pub fn to_simplified(text: &str) -> String {
    let pairs: Vec<char> = ZH_PAIRS.chars().collect();
    text.chars()
        .map(|c| match pairs.iter().position(|&x| x == c) {
            Some(i) if i % 2 == 0 => pairs[i + 1],
            _ => c,
        })
        .collect()
}

impl Profile {
    pub fn of(text: &str) -> Profile {
        let pairs: Vec<char> = ZH_PAIRS.chars().collect();
        let mut p = Profile::default();
        for c in text.chars() {
            match c as u32 {
                0x3041..=0x30FF if c != '・' && c != 'ー' => p.kana += 1,
                0x3400..=0x4DBF | 0x4E00..=0x9FFF => p.han += 1,
                0xAC00..=0xD7AF => p.hangul += 1,
                _ if c.is_alphabetic() && (c as u32) < 0x0250 => p.latin += 1,
                _ => {}
            }
            if let Some(i) = pairs.iter().position(|&x| x == c) {
                if i % 2 == 0 {
                    p.traditional += 1;
                } else {
                    p.simplified += 1;
                }
            }
        }
        p
    }

    pub fn letters(&self) -> usize {
        self.latin + self.kana + self.han + self.hangul
    }

    /// The primary language the text looks like, among those Kensa can judge.
    /// Returns None when the text is too short to decide.
    pub fn guess(&self) -> Option<&'static str> {
        let total = self.letters();
        if total < 80 {
            return None;
        }
        let share = |n: usize| n as f64 / total as f64;
        // Japanese mixes kana with kanji and often Latin words; an untranslated page keeps a
        // little kana from the menus, so kana alone is not enough when Latin dominates.
        let cjk = self.kana + self.han;
        if share(self.kana) >= 0.15 || (share(self.kana) >= 0.05 && cjk >= self.latin) {
            Some("ja")
        } else if share(self.han) >= 0.5 {
            Some("zh")
        } else if share(self.hangul) >= 0.5 {
            Some("ko")
        } else if share(self.latin) >= 0.6 {
            Some("latin")
        } else {
            None
        }
    }

    /// The Chinese script the text is written in, when it is clear.
    pub fn zh_script(&self) -> Option<ZhScript> {
        if self.traditional >= 5 && self.traditional >= 2 * self.simplified {
            Some(ZhScript::Traditional)
        } else if self.simplified >= 5 && self.simplified >= 2 * self.traditional {
            Some(ZhScript::Simplified)
        } else {
            None
        }
    }
}

/// True when a declared primary language agrees with what the text looks like.
pub fn agrees(declared: &str, guess: &str) -> bool {
    match guess {
        "latin" => !matches!(declared, "ja" | "zh" | "ko"),
        g => g == declared,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_good_codes() {
        assert_eq!(parse("zh-Hant-TW").unwrap().script.as_deref(), Some("Hant"));
        assert_eq!(parse("ja").unwrap().language, "ja");
        assert_eq!(parse("fr-FR").unwrap().region.as_deref(), Some("FR"));
    }

    #[test]
    fn rejects_common_mistakes() {
        assert!(parse("jp").unwrap_err().contains("\"ja\""));
        assert!(parse("en-UK").unwrap_err().contains("GB"));
        assert!(parse("ja-JA").is_err());
        assert!(parse("xx").is_err());
    }

    #[test]
    fn tells_traditional_from_simplified() {
        let t = Profile::of("這個筆記本是用亞麻紙做的，價錢優惠，歡迎選購。我們的禮物包裝很實用。");
        let s = Profile::of("这个笔记本是用亚麻纸做的，价钱优惠，欢迎选购。我们的礼物包装很实用。");
        assert_eq!(t.zh_script(), Some(ZhScript::Traditional));
        assert_eq!(s.zh_script(), Some(ZhScript::Simplified));
    }

    #[test]
    fn untranslated_page_with_japanese_menus_is_not_japanese() {
        let text = format!(
            "ホーム 紙と文具 お茶 暮らしの道具 私たちについて 読みもの カート {}",
            "Un bon carnet se choisit d'abord à la main, puis on regarde le papier et la reliure. ".repeat(6)
        );
        assert_eq!(Profile::of(&text).guess(), Some("latin"));
    }

    #[test]
    fn guesses_japanese_even_with_many_kanji() {
        let text = "麻の便箋は、手書きの手紙にぴったりです。".repeat(6);
        assert_eq!(Profile::of(&text).guess(), Some("ja"));
    }
}
