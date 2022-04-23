mod style;

use std::{
    fs::File,
    io::{Read, Write},
    path::PathBuf, ops::Sub,
};
use std::thread::JoinHandle;

use chrono::{Date, Local, LocalResult, TimeZone, Datelike};
use directories::ProjectDirs;
use eframe::{
    egui::{self, TextEdit, Key}, epi,
};
use eframe::epi::App;
use walkdir::WalkDir;

#[derive(Copy, Clone, Debug)]
enum BufferId {
    Date(Date<Local>),
}

impl Default for BufferId {
    fn default() -> Self {
        Self::Date(Local::now().date())
    }
}

impl BufferId {
    fn today() -> Self {
        BufferId::default()
    }

    fn yesterday() -> Self {
        Self::Date(Local::now().date().sub(chrono::Duration::days(1)))
    }

    fn filepath(&self) -> PathBuf {
        match self {
            Self::Date(dt) => {
                let mut path = PathBuf::new();
                path.push(dt.year().to_string());
                path.push(dt.month().to_string());
                path.push(dt.day().to_string());
                path
            }
        }
    }
}

#[derive(Debug, Default)]
struct SavedFiles {}

impl SavedFiles {

    fn root_dir(&self) -> PathBuf {
        if let Some(project_dirs) = ProjectDirs::from("com", "marschium", "NNNNotes") {
            project_dirs.data_dir().into()
        } else {
            ".".into()
        }
    }

    fn save(&self, id: &BufferId, buf: &String) -> Result<(), std::io::Error> {
        let mut path = self.root_dir();
        path.push(id.filepath());
        std::fs::create_dir_all(path.parent().unwrap())?;
        match File::create(path) {
            Ok(mut f) => {
                f.write_all(&buf.as_bytes())?;
                Ok(())
            }
            Err(e) => Err(e),
        }
    }
    
    fn load(&mut self, id: &BufferId, buf: &mut String) -> Result<(), std::io::Error> {
        let mut path = self.root_dir();
        path.push(id.filepath());
        match File::open(path) {
            Ok(mut f) => {
                buf.clear();
                f.read_to_string(buf)?;
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    fn has(&self, id: &BufferId) -> bool {
        let mut path = self.root_dir();
        path.push(id.filepath());
        path.exists()
    }
}

struct BackgroundState {
    j: JoinHandle<()>
}

impl BackgroundState {
    fn run(frame: epi::Frame) -> Self {
        let j = std::thread::spawn(move || {
            loop {
                std::thread::sleep(std::time::Duration::from_secs(1));
                frame.request_repaint();
            }
        });
        Self {
            j
        }
    }
}

fn format_duration(duration: &chrono::Duration) -> String {
    if duration < &chrono::Duration::seconds(60) {
        format!("{}s", duration.num_seconds())
    }
    else {
        "a long time".to_owned()
    }
}


#[derive(Default)]
struct MyEguiApp {
    buffer_id: BufferId,
    buffer: String,
    available_buffers: Vec<BufferId>,
    saved_files: SavedFiles,
    last_saved: Option<chrono::DateTime<Local>>,
    background_state: Option<BackgroundState>
}

impl MyEguiApp {
    pub fn load() -> Self {
        let mut s = Self::default();
        let copy_from_yesterday = !s.saved_files.has(&BufferId::today());
        if copy_from_yesterday {
            let _ = s.saved_files.load(&BufferId::yesterday(), &mut s.buffer);
            let _ = s.saved_files.save(&BufferId::today(), &s.buffer);
        }
        else {
            let _ = s.saved_files.load(&BufferId::today(), &mut s.buffer);
        }

        s.update_available_buffers();
        s
    }

    fn update_available_buffers(&mut self) {
        self.available_buffers.clear();
        for entry in WalkDir::new(self.saved_files.root_dir())
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.path().is_file() {
                let mut components = entry.path().components().into_iter().rev();
                let day = components
                    .next()
                    .and_then(|x| x.as_os_str().to_str())
                    .and_then(|x| x.parse::<u32>().ok());
                let month = components
                    .next()
                    .and_then(|x| x.as_os_str().to_str())
                    .and_then(|x| x.parse::<u32>().ok());
                let year = components
                    .next()
                    .and_then(|x| x.as_os_str().to_str())
                    .and_then(|x| x.parse::<i32>().ok());

                match (year, month, day) {
                    (Some(year), Some(month), Some(day)) => {
                        let d = Local.ymd_opt(year, month, day);
                        match d {
                            LocalResult::Single(d) => {
                                self.available_buffers.push(BufferId::Date(d));
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    

    fn swap_to_buffer(&mut self, id: &BufferId) {
        let _ = self.saved_files.save(&self.buffer_id, &self.buffer);
        self.buffer_id = *id;
        let _ = self.saved_files.load(&self.buffer_id, &mut self.buffer);
    }
}

impl epi::App for MyEguiApp {
    fn name(&self) -> &str {
        "My egui App"
    }

    fn update(&mut self, ctx: &egui::Context, frame: &epi::Frame) {
        if self.background_state.is_none() {
            self.background_state = Some(BackgroundState::run(frame.clone()));
        }

        match self.last_saved {
            Some(v) => {
                if Local::now() - v > chrono::Duration::seconds(5) {
                    self.last_saved = Some(Local::now());
                    let _ = self.saved_files.save(&self.buffer_id, &self.buffer);                    
                }
            },
            None => {
                self.last_saved = Some(Local::now());
                let _ = self.saved_files.save(&self.buffer_id, &self.buffer);
            }
        }

        let command_key_down = ctx.input().modifiers.command;
        if command_key_down && ctx.input().key_pressed(Key::T) {
            self.swap_to_buffer(&BufferId::today());
        }
        if command_key_down && ctx.input().key_pressed(Key::S) {
            let _ =  self.saved_files.save(&self.buffer_id, &self.buffer);
        }

        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            if let Some(last_saved) = self.last_saved {
                let time_since_last_save = Local::now() - last_saved;
                ui.label(format!("Last Save: {} ago" , format_duration(&time_since_last_save)));
            }
            ui.centered_and_justified(|ui| {
                ui.label(self.buffer_id.filepath().to_str().unwrap_or("???"));
            });            
        });
        egui::SidePanel::left("buffers").show(ctx, |ui| {
            for id in self.available_buffers.clone() {
                if ui.button(id.filepath().to_str().unwrap()).clicked() {
                    self.swap_to_buffer(&id)
                }
            }
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            let mut layouter = |ui: &egui::Ui, string: &str, wrap_width: f32| {
                let mut layout_job: egui::text::LayoutJob = style::highlight(string);
                layout_job.wrap_width = wrap_width;
                ui.fonts().layout_job(layout_job)
            };
            ui.add_sized(
                ui.available_size(),
                TextEdit::multiline(&mut self.buffer).layouter(&mut layouter),
            );
        });
    }
}

fn main() {
    let app = MyEguiApp::load();
    let native_options = eframe::NativeOptions::default();
    // TODO start thread for autosave and updating the tree
    eframe::run_native(Box::new(app), native_options);
}
