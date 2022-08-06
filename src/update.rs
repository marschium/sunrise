use eframe::egui::special_emojis::GITHUB;

pub const VERSION: &'static str = env!("CARGO_PKG_VERSION");
pub const GITHUB_VERSION: Option<&'static str> = option_env!("build_version");

pub fn current_version() -> semver::Version {
    let parsed = match GITHUB_VERSION {
        Some(v) => semver::Version::parse(&v[1..]), // remove leading 'v'
        None => semver::Version::parse(VERSION)
    };
    parsed.unwrap_or(semver::Version::new(1, 0, 0))
}

pub fn update_available() -> bool {
    let our_version = semver::Version::parse(VERSION).unwrap();
    print!("{:?}", our_version);
    false
}