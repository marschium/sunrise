use std::{
    fs::File,
    io::{Read, Write},
    path::{Path, PathBuf},
};

pub struct LinesWithEndings<'a> {
    input: &'a str,
}

impl<'a> LinesWithEndings<'a> {
    pub fn from(input: &'a str) -> LinesWithEndings<'a> {
        LinesWithEndings { input: input }
    }
}

impl<'a> Iterator for LinesWithEndings<'a> {
    type Item = &'a str;

    #[inline]
    fn next(&mut self) -> Option<&'a str> {
        if self.input.is_empty() {
            return None;
        }
        let split = self
            .input
            .find('\n')
            .map(|i| i + 1)
            .unwrap_or(self.input.len());
        let (line, rest) = self.input.split_at(split);
        self.input = rest;
        Some(line)
    }
}

enum Error {
    IoError(std::io::Error),
}

use chrono::{DateTime, Datelike, Local};
use eframe::{
    egui::{self, TextEdit, TextFormat},
    epaint::{
        text::{LayoutJob, LayoutSection},
        Color32, FontFamily, FontId,
    },
    epi,
};

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
    fn save_buffer(&self) -> Result<(), std::io::Error> {
        let path = self.buffer_id.filepath();
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
        match File::open(self.buffer_id.filepath()) {
            Ok(mut f) => {
                self.buffer.clear();
                f.read_to_string(&mut self.buffer)?;
                Ok(())
            }
            Err(e) => Err(e),
        }
    }
}

fn as_byte_range(whole: &str, range: &str) -> std::ops::Range<usize> {
    let whole_start = whole.as_ptr() as usize;
    let range_start = range.as_ptr() as usize;
    assert!(whole_start <= range_start);
    assert!(range_start + range.len() <= whole_start + whole.len());
    let offset = range_start - whole_start;
    offset..(offset + range.len())
}

fn layout_test(text: &str) -> LayoutJob {
    let mut job = LayoutJob::default();
    job.text = text.into();
    let mut color = Color32::WHITE;
    for line in LinesWithEndings::from(text) {
        job.sections.push(LayoutSection {
            byte_range: as_byte_range(text, line),
            leading_space: 0.0,
            format: TextFormat {
                font_id: FontId::new(14.0, FontFamily::Proportional),
                color,
                ..Default::default()
            },
        });

        match color {
            Color32::WHITE => color = Color32::RED,
            _ => color = Color32::WHITE,
        }
    }

    job
}

impl epi::App for MyEguiApp {
    fn name(&self) -> &str {
        "My egui App"
    }

    fn update(&mut self, ctx: &egui::Context, frame: &epi::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            if ui.button("Save").clicked() {
                self.save_buffer(); // TODO show error
            }
            if ui.button("Load").clicked() {
                self.load_buffer(); // TODO show error
            }

            let mut layouter = |ui: &egui::Ui, string: &str, wrap_width: f32| {
                let mut layout_job: egui::text::LayoutJob = layout_test(string);
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
