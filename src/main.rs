mod style;

use std::{
    fs::File,
    io::{Read, Write},
    path::{PathBuf},
};

use chrono::{DateTime, Datelike, Local};
use directories::ProjectDirs;
use eframe::{
    egui::{self, TextEdit},
    epi,
};
use walkdir::WalkDir;

enum BufferId {
    Date(DateTime<Local>),
}

impl Default for BufferId {
    fn default() -> Self {
        Self::Date(Local::now())
    }
}

impl BufferId {
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
}

impl MyEguiApp {

    fn root_dir(&self) -> PathBuf {
        if let Some(project_dirs) = ProjectDirs::from("com", "marschium", "notez") {
            project_dirs.data_dir().into()
        }
        else {
            ".".into()
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
}

impl epi::App for MyEguiApp {
    fn name(&self) -> &str {
        "My egui App"
    }

    fn update(&mut self, ctx: &egui::Context, frame: &epi::Frame) {
        egui::SidePanel::left("Hello World").show(ctx, |ui| {
            // TODO cache and only walk when something changes
            for entry in WalkDir::new(self.root_dir()).into_iter().filter_map(|e| e.ok()) {
                if entry.path().is_file() {
                    ui.label(entry.path().to_str().unwrap_or("???"));
                }
            }
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            if ui.button("Save").clicked() {
                self.save_buffer(); // TODO show error
            }
            if ui.button("Load").clicked() {
                self.load_buffer(); // TODO show error
            }

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
    let app = MyEguiApp::default();
    // TODO load the current date entry if exists. create new one if not
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(Box::new(app), native_options);
}
