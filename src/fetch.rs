//! HTTP fetching. Redirects are not followed automatically, so chains can be reported.

use std::time::Duration;

pub const USER_AGENT: &str =
    concat!("kensa/", env!("CARGO_PKG_VERSION"), " (+https://github.com/thibaultbordrie-cloud/kensa)");

#[derive(Debug, Clone)]
pub struct Response {
    pub status: u16,
    pub content_type: String,
    pub location: Option<String>,
    pub x_robots: String,
    pub body: String,
}

pub struct Fetcher {
    client: reqwest::blocking::Client,
    max_bytes: usize,
}

impl Fetcher {
    pub fn new(timeout_secs: u64) -> anyhow::Result<Fetcher> {
        let client = reqwest::blocking::Client::builder()
            .user_agent(USER_AGENT)
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(timeout_secs))
            .build()?;
        Ok(Fetcher { client, max_bytes: 5_000_000 })
    }

    /// One GET, no redirect. Network errors come back as status 0 with the error as body.
    pub fn get(&self, url: &str) -> Response {
        match self.client.get(url).send() {
            Ok(resp) => {
                let header =
                    |name: &str| resp.headers().get(name).and_then(|v| v.to_str().ok()).unwrap_or("").to_string();
                let status = resp.status().as_u16();
                let content_type = header("content-type");
                let x_robots = header("x-robots-tag");
                let location = Some(header("location")).filter(|l| !l.is_empty());
                let text = resp.text().unwrap_or_default();
                let mut body = text;
                if body.len() > self.max_bytes {
                    let mut cut = self.max_bytes;
                    while !body.is_char_boundary(cut) {
                        cut -= 1;
                    }
                    body.truncate(cut);
                }
                Response { status, content_type, location, x_robots, body }
            }
            Err(e) => Response {
                status: 0,
                content_type: String::new(),
                location: None,
                x_robots: String::new(),
                body: {
                    // Keep the whole cause chain: "error sending request" alone does not help.
                    let mut msg = e.to_string();
                    let mut src = std::error::Error::source(&e);
                    while let Some(cause) = src {
                        msg.push_str(": ");
                        msg.push_str(&cause.to_string());
                        src = cause.source();
                    }
                    msg
                },
            },
        }
    }
}
