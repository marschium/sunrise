use eframe::{epaint::{text::{LayoutJob, LayoutSection}, Color32, FontFamily, FontId}, egui::TextFormat};

// https://stackoverflow.com/questions/40455997/iterate-over-lines-in-a-string-including-the-newline-characters
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

fn as_byte_range(whole: &str, range: &str) -> std::ops::Range<usize> {
    let whole_start = whole.as_ptr() as usize;
    let range_start = range.as_ptr() as usize;
    assert!(whole_start <= range_start);
    assert!(range_start + range.len() <= whole_start + whole.len());
    let offset = range_start - whole_start;
    offset..(offset + range.len())
}

pub fn highlight(text: &str) -> LayoutJob {
    // TODO nom here?
    let mut job = LayoutJob::default();
    job.text = text.into();
    
    for line in LinesWithEndings::from(text) {
        let mut format = TextFormat {
            font_id: FontId::new(14.0, FontFamily::Proportional),
            color: Color32::DARK_GRAY,
            ..Default::default()
        };

        let trimmed = line.trim();
        if trimmed.starts_with("[/]") {
            format.color = Color32::GREEN;
        }
        else if trimmed.starts_with("[x]") {
            format.color = Color32::RED;
        }
        else if trimmed.starts_with("#") {
            format.font_id = FontId::new(18.0, FontFamily::Proportional);
        }
        job.sections.push(LayoutSection {
            byte_range: as_byte_range(text, line),
            leading_space: 0.0,
            format,
        });
    }

    job
}