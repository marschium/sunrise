use std::{
    fs::{File, Permissions},
    io::Write,
    path::{Path, PathBuf},
    process::{Command, exit},
    sync::{Arc, Condvar, Mutex},
    thread::{current, JoinHandle},
};

#[cfg(target_os = "linux")]
use std::os::unix::{prelude::PermissionsExt, process::CommandExt};

pub const VERSION: &'static str = env!("CARGO_PKG_VERSION");
pub const GITHUB_VERSION: Option<&'static str> = option_env!("build_version");
pub const UPDATE_URL: &'static str =
    "https://api.github.com/repos/marschium/sunrise/releases/latest";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LatestVersion {
    pub ver: semver::Version,
    pub url: String,
}

impl LatestVersion {
    pub fn newer_than_current(&self) -> bool {
        self.ver > current_version()
    }
}

pub fn current_version() -> semver::Version {
    let parsed = match GITHUB_VERSION {
        Some(v) => semver::Version::parse(&v[1..]), // remove leading 'v'
        None => semver::Version::parse(VERSION),
    };
    parsed.unwrap_or(semver::Version::new(1, 0, 0))
}

pub fn latest_version() -> Option<LatestVersion> {
    let client = reqwest::blocking::Client::builder()
        .user_agent("MYAPP/1.0")
        .build()
        .unwrap();
    if let Ok(resp) = client.get(UPDATE_URL).send() {
        //println!("{:?}", resp.text());
        if let Ok(j) = resp.json::<serde_json::Value>() {
            let tag_name = j["tag_name"].as_str()?;
            let ver = semver::Version::parse(&tag_name[1..]).ok()?;
            let asset = (j["assets"].as_array())?.get(0)?;
            let url =  
            if cfg!(target_os = "linux") {
                let asset = (j["assets"].as_array())?.iter().find(|x| !x["name"].as_str().unwrap_or("").ends_with(".exe"));
                asset?["browser_download_url"].as_str()
            }
            else {
                let asset = (j["assets"].as_array())?.iter().find(|x| x["name"].as_str().unwrap_or("").ends_with(".exe"));
                asset?["browser_download_url"].as_str()
            }?;
            Some(LatestVersion {
                ver,
                url: url.to_string(),
            })
        } else {
            None
        }
    } else {
        None
    }
}


#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateServiceState {
    Checking,
    Unavailable,
    UpdateAvailable(LatestVersion),
    Downloading,
    Downloaded
}

pub struct UpdateService {
    state_pair: Arc<(Mutex<UpdateServiceState>, Condvar)>,
    j: JoinHandle<()>,
}

impl UpdateService {
    pub fn start() -> Self {
        let state_pair = Arc::new((Mutex::new(UpdateServiceState::Checking), Condvar::new()));
        let j = {
            let state = Arc::clone(&state_pair);
            let j = std::thread::spawn(move || {
                let (state, cond) = &*state;

                let updated_version = if let Some(v) = latest_version() {
                    let mut current_state = state.lock().unwrap();
                    (*current_state) = UpdateServiceState::UpdateAvailable(v.clone());
                    Some(v)
                }
                else {
                    let mut current_state = state.lock().unwrap();
                    (*current_state) = UpdateServiceState::Unavailable;
                    None
                };

                // TODO stream with progress
                if let Some(updated_version) = updated_version {
                    if updated_version.newer_than_current() {
                        {
                            let mut current_state = state.lock().unwrap();
                            *current_state = UpdateServiceState::Downloading;                        
                        }
    
                        let client = reqwest::blocking::Client::builder()
                            .user_agent("MYAPP/1.0")
                            .build()
                            .unwrap();
                        let res = client.get(updated_version.url.clone()).send().unwrap();
                        let mut f = File::create("update").unwrap();
                        f.write_all(&res.bytes().unwrap());
    
    
                        {
                            let mut current_state = state.lock().unwrap();
                            *current_state = UpdateServiceState::Downloaded;                        
                        }
                    }                    
                }
                
            });
            j
        };

        Self { state_pair, j }
    }

    pub fn state(&self) -> UpdateServiceState {
        let l = self.state_pair.0.lock().unwrap();
        l.clone()
    }

    pub fn apply(&self) {
        #[cfg(target_os = "linux")]
        {
            if self.state() == UpdateServiceState::Downloaded {
                let new_exe = "./update";
                std::fs::set_permissions(new_exe, Permissions::from_mode(0o755));
                let exe = std::env::current_exe().unwrap();
                let this_exe = exe.to_str().unwrap();
                Command::new("sh")
                    .arg("-c")
                    .arg(format!("mv {new_exe} {this_exe} && {this_exe}"))
                    .exec();
            }
        }
        
        #[cfg(target_os = "windows")]
        {
            if self.state() == UpdateServiceState::Downloaded {
                let new_exe = "./update";
                let pid = std::process::id();
                let exe = std::env::current_exe().unwrap();
                let this_exe = exe.to_str().unwrap();
                let e = Command::new("cmd")
                    .arg("/C")
                    .arg(format!("taskkill /F /PID {pid} && waitfor NEVERHAPPENINGPAL /t 10 2>NUL & move /y {new_exe} {this_exe} && {this_exe}"))
                    .output()
                    .expect("Update command failed");
                println!("{}", String::from_utf8_lossy(&e.stdout));
                println!("{}", String::from_utf8_lossy(&e.stderr));
            }
        }
    }
}
