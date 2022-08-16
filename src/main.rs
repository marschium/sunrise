#![windows_subsystem = "windows"]

mod note_tree;
mod style;
mod update;

use std::{
    env,
    fs::File,
    io::{Read, Write},
    ops::Sub,
    path::PathBuf,
};

use chrono::{Date, Datelike, Local, LocalResult, TimeZone};
use directories::ProjectDirs;
use eframe::{
    egui::{self, text_edit::CursorRange, Event, Key, Layout, TextEdit},
    epi,
};
use note_tree::show_note_tree;
use regex::Regex;
use style::CachedLayoutJobBuilder;
use update::{current_version, UpdateService, UpdateServiceState};
use walkdir::WalkDir;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct BufferId {
    date: Date<Local>,
}

impl Default for BufferId {
    fn default() -> Self {
        Self {
            date: Local::now().date(),
        }
    }
}

impl BufferId {
    fn new(date: Date<Local>) -> Self {
        Self { date }
    }

    fn today() -> Self {
        BufferId::default()
    }

    fn yesterday() -> Self {
        Self {
            date: Local::now().date().sub(chrono::Duration::days(1)),
        }
    }

    fn prev(&self) -> Self {
        Self {
            date: self.date.sub(chrono::Duration::days(1)),
        }
    }

    fn filepath(&self) -> PathBuf {
        let dt = self.date;
        let mut path = PathBuf::new();
        path.push(dt.year().to_string());
        path.push(dt.month().to_string());
        path.push(dt.day().to_string());
        path
    }
}

#[derive(Debug, Default)]
struct SavedFiles {}

impl SavedFiles {
    fn root_dir(&self) -> PathBuf {
        if let Some(project_dirs) = ProjectDirs::from("com", "marschium", "sunrise") {
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

struct MyEguiApp {
    buffer_id: BufferId,
    buffer: String,
    available_buffers: Vec<BufferId>,
    saved_files: SavedFiles,
    saved: bool,
    update_service: UpdateService,
    cursor: Option<CursorRange>,
    last_changed: Option<chrono::DateTime<Local>>,
    highlight_cache: CachedLayoutJobBuilder,
}

impl Default for MyEguiApp {
    fn default() -> Self {
        Self {
            buffer_id: Default::default(),
            buffer: Default::default(),
            available_buffers: Default::default(),
            saved_files: Default::default(),
            saved: Default::default(),
            update_service: UpdateService::start(),
            cursor: Default::default(),
            last_changed: Default::default(),
            highlight_cache: Default::default(),
        }
    }
}

impl MyEguiApp {
    pub fn load(demo: bool) -> Self {
        //

        let mut s = Self::default();
        let copy_from_previous = !s.saved_files.has(&BufferId::today());
        if copy_from_previous {
            let mut id = BufferId::yesterday();
            let mut i = 0;
            while !s.saved_files.has(&id) && i < 14 {
                id = id.prev();
                i += 1;
            }

            if s.saved_files.has(&id) {
                let _ = s.saved_files.load(&id, &mut s.buffer);
                let _ = s.saved_files.save(&BufferId::today(), &s.buffer);
            }
        } else {
            let _ = s.saved_files.load(&BufferId::today(), &mut s.buffer);
        }

        s.update_available_buffers();
        if demo {
            s.buffer = r"# Header
[ ] Something todo
[/] Something done
[x] Something cancelled
`monospaced something`
regular text
https://google.com/about.html?arg=hello%20world

"
            .to_string();
        }
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
                                self.available_buffers.push(BufferId::new(d));
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

    // replace first occurance of 'find' on the current line
    fn replace_on_current_line(&mut self, start_pos: usize, find: &str, replace: &str) -> bool {
        let substr = self.buffer.get(0..start_pos).unwrap();
        let search_start = substr.rfind("\n").unwrap_or(0);
        let substr = self.buffer.get(search_start..start_pos).unwrap();
        match substr.rfind(find) {
            Some(i) => {
                self.buffer
                    .replace_range(i + search_start..i + search_start + find.len(), replace);
                true
            }
            None => false,
        }
    }

    fn replace_task_for_cursor(&mut self, cursor_pos: usize) {
        let mut changed = self.replace_on_current_line(cursor_pos, "[x]", "[/]");
        if !changed {
            changed = self.replace_on_current_line(cursor_pos, "[/]", "[x]");
        }
        if !changed {
            changed = self.replace_on_current_line(cursor_pos, "[]", "[ ]");
        }
        if !changed {
            changed = self.replace_on_current_line(cursor_pos, "[ ]", "[/]");
        }
        if !changed {
            let substr = self.buffer.get(0..cursor_pos).unwrap();
            let end_of_prev_line = substr.rfind("\n").unwrap_or(0);
            self.buffer.insert_str(
                if end_of_prev_line > 0 {
                    end_of_prev_line + 1
                } else {
                    0
                },
                "[ ] ",
            );
        }
    }
}

impl epi::App for MyEguiApp {
    fn name(&self) -> &str {
        "sunrise"
    }

    fn update(&mut self, ctx: &egui::Context, frame: &epi::Frame) {
        ctx.set_visuals(egui::Visuals::dark());

        if !self.saved {
            let recent_edit = match self.last_changed {
                Some(last_changed) => (Local::now() - last_changed > chrono::Duration::seconds(5)),
                _ => false,
            };
            if recent_edit {
                self.saved = true;
                let _ = self.saved_files.save(&self.buffer_id, &self.buffer);
            }
        }

        let mut any_key_pressed = false;
        for event in ctx.input().events.clone() {
            if !any_key_pressed {
                any_key_pressed = matches!(event, Event::Text(..))
                    || matches!(event, Event::Paste(..))
                    || matches!(event, Event::Key { .. });
            }
            match event {
                Event::Key {
                    key: Key::M,
                    pressed: true,
                    modifiers,
                } => {
                    if modifiers.command {
                        match self.cursor {
                            Some(cursor) => {
                                self.replace_task_for_cursor(cursor.primary.ccursor.index);
                            }
                            None => {}
                        }
                    }
                }
                Event::Key {
                    key: Key::T,
                    pressed: true,
                    modifiers,
                } => {
                    if modifiers.command {
                        self.swap_to_buffer(&BufferId::today());
                    }
                }
                Event::Key {
                    key: Key::S,
                    pressed: true,
                    modifiers,
                } => {
                    if modifiers.command {
                        self.saved = true;
                        let _ = self.saved_files.save(&self.buffer_id, &self.buffer);
                    }
                }
                _ => {}
            }
        }

        egui::TopBottomPanel::top("top").show(ctx, |ui| {

            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::left_to_right(), |ui| {
                    if self.saved {
                        ui.label("Saved");
                    } else {
                        ui.label("Not Saved");
                    }
                    ui.centered_and_justified(|ui| {
                        ui.label(self.buffer_id.filepath().to_str().unwrap_or("???"));
                    });
                    ui.with_layout(egui::Layout::right_to_left(), |ui| {
                        ui.add_space(8.0);
                        ui.label(format!("v{}", current_version()));

                        let current_update_status = self.update_service.state();
                        match current_update_status {
                            UpdateServiceState::Downloaded => {
                                if ui.button("Update").clicked() {
                                    self.update_service.apply();
                                }
                            },
                            _ => {}
                        }
                    });

                    
                });
            });
        });
        egui::SidePanel::left("buffers").show(ctx, |ui| {
            if let Some(buffer_id) = show_note_tree(&self.available_buffers, ui) {
                self.swap_to_buffer(&buffer_id);
                any_key_pressed = true;
            }
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            let mut layouter = |ui: &egui::Ui, string: &str, wrap_width: f32| {
                if any_key_pressed {
                    self.highlight_cache.clear();
                }
                let mut layout_job = self.highlight_cache.highlight(string);
                layout_job.wrap_width = wrap_width;
                ui.fonts().layout_job(layout_job)
            };

            let mut text_changed = false;
            let layout = Layout::centered_and_justified(ui.layout().main_dir());
            ui.allocate_ui_with_layout(ui.available_size(), layout, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let output = TextEdit::multiline(&mut self.buffer)
                        .layouter(&mut layouter)
                        .lock_focus(true)
                        .show(ui);
                    text_changed = output.response.changed();
                    self.cursor = output.cursor_range;
                    // TODO if cursor was clicked, did we click anything interesting?
                    if output.response.double_clicked() {
                        // cursor will have already been moved so just use that position
                        if let Some(cpos) = output.cursor_range {
                            // scan the text here and see if cursor is in the middle of a hyperlink
                            let cur = cpos.primary.ccursor.index;
                            let c = &output.galley.text()[..cur];
                            let front = output.galley.text()[..cur]
                                .rfind(|ch: char| ch.is_whitespace() || ch == '\n');
                            let back = output.galley.text()[cur..]
                                .find(|ch: char| ch.is_whitespace() || ch == '\n');
                            if let (Some(front), Some(back)) = (front, back) {
                                let re = Regex::new(r".?://.?").unwrap(); // TODO cache this
                                let selected = &output.galley.text()[front..cur + back];
                                if re.is_match(selected) {
                                    webbrowser::open(selected);
                                }
                            }
                        }
                    }
                });
            });

            if text_changed {
                self.saved = false;
                self.last_changed = Some(Local::now());
            }
        });
    }
}

fn main() {
    let args: Vec<_> = env::args().collect();
    let app = MyEguiApp::load(args.get(1) == Some(&"--demo".to_string()));
    let mut native_options = eframe::NativeOptions::default();
    native_options.maximized = true;
    eframe::run_native(Box::new(app), native_options);
}
