use std::thread::current;

use eframe::egui::special_emojis::GITHUB;
use reqwest::Client;

pub const VERSION: &'static str = env!("CARGO_PKG_VERSION");
pub const GITHUB_VERSION: Option<&'static str> = option_env!("build_version");
pub const UPDATE_URL: &'static str = "https://api.github.com/repos/marschium/AirhornNotes/releases/latest";

pub struct LatestVersion {
    pub ver: semver::Version,
    pub url: String
}

impl LatestVersion {
    pub fn newer_than_current(&self) -> bool {
        self.ver > current_version()
    }
}

pub fn current_version() -> semver::Version {
    let parsed = match GITHUB_VERSION {
        Some(v) => semver::Version::parse(&v[1..]), // remove leading 'v'
        None => semver::Version::parse(VERSION)
    };
    parsed.unwrap_or(semver::Version::new(1, 0, 0))
}

pub fn latest_version() -> Option<LatestVersion> {
    let client = reqwest::blocking::Client::builder().user_agent("MYAPP/1.0").build().unwrap();
    if let Ok(resp) = client.get(UPDATE_URL).send() {
        //println!("{:?}", resp.text());
        if let Ok(j) = resp.json::<serde_json::Value>() {
            let tag_name = j["tag_name"].as_str()?;
            let ver = semver::Version::parse(&tag_name[1..]).ok()?;
            let asset = (j["assets"].as_array())?.get(0)?;
            let url = asset["browser_download_url"].as_str()?;
            Some(LatestVersion { ver, url: url.to_string() })
            // TODO pick the correct asset based on OS
        }
        else {
            None
        }
    }
    else {
        None
    }
}