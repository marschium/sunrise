mod style;

use std::{
    fs::File,
    io::{Read, Write},
    path::PathBuf,
};

use chrono::{Date, DateTime, Datelike, Local, LocalResult, NaiveDate, TimeZone};
use directories::ProjectDirs;
use eframe::{
    egui::{self, TextEdit},
    epi,
};
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

#[derive(Default)]
struct MyEguiApp {
    buffer_id: BufferId,
    buffer: String,
    available_buffers: Vec<BufferId>,
}

impl MyEguiApp {
    pub fn load() -> Self {
        let mut s = Self::default();
        let _ = s.load_buffer();
        s.update_available_buffers();
        s
    }

    fn root_dir(&self) -> PathBuf {
        if let Some(project_dirs) = ProjectDirs::from("com", "marschium", "notez") {
            project_dirs.data_dir().into()
        } else {
            ".".into()
        }
    }

    fn update_available_buffers(&mut self) {
        self.available_buffers.clear();
        for entry in WalkDir::new(self.root_dir())
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

    fn save_buffer(&self) -> Result<(), std::io::Error> {
        let mut path = self.root_dir();
        path.push(self.buffer_id.filepath());
        std::fs::create_dir_all(path.parent().unwrap())?;
        match File::create(path) {
            Ok(mut f) => {
                f.write_all(&self.buffer.as_bytes())?;
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    fn load_buffer(&mut self) -> Result<(), std::io::Error> {
        let mut path = self.root_dir();
        path.push(self.buffer_id.filepath());
        match File::open(path) {
            Ok(mut f) => {
                self.buffer.clear();
                f.read_to_string(&mut self.buffer)?;
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    fn swap_to_buffer(&mut self, id: &BufferId) {
        let _ = self.save_buffer();
        self.buffer_id = *id;
        let _ = self.load_buffer();
    }
}

impl epi::App for MyEguiApp {
    fn name(&self) -> &str {
        "My egui App"
    }

    fn update(&mut self, ctx: &egui::Context, frame: &epi::Frame) {
        egui::SidePanel::left("buffers").show(ctx, |ui| {
            for id in self.available_buffers.clone() {
                if ui.button(id.filepath().to_str().unwrap()).clicked() {
                    self.swap_to_buffer(&id)
                }
            }
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            // TODO autosave
            // TODO make these hotkeys
            ui.horizontal(|ui| {
                if ui.button("Save").clicked() {
                    let _ = self.save_buffer(); // TODO show error
                }
                if ui.button("Load").clicked() {
                    let _ = self.load_buffer(); // TODO show error
                }
                if ui.button("Today").clicked() {
                    self.swap_to_buffer(&BufferId::today())
                }
            });

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
    // TODO load the current date entry if exists. create new one if not
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(Box::new(app), native_options);
}
