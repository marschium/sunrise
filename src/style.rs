use eframe::{
    egui::{TextFormat},
    epaint::{
        text::{LayoutJob, LayoutSection},
        Color32, FontFamily, FontId,
    },
};
use nom::{
    bytes::complete::{tag, take_until, take_while},
    character::{complete::{newline, not_line_ending, space0, alphanumeric1, line_ending, satisfy}, is_alphabetic},
    sequence::{tuple, delimited, terminated},
    IResult, branch::alt, multi::{fold_many1, many1}, combinator::not,
};

struct Style {
    pub look: TextFormat,
    pub len: usize,
}

fn header(s: &str) -> IResult<&str, Style> {
    let mut inner = tuple((space0, many1(tag("#")), not_line_ending, newline));
    let (extra, span) = inner(s)?;
    let num_tags = span.1.len() as f32;
    let mut font_size = 26.0 - (4.0 * (num_tags - 1.0));
    if font_size < 16.0 {
        font_size = 16.0
    }
    Ok((
        extra,
        Style {
            look: TextFormat {
                font_id: FontId::new(font_size, FontFamily::Proportional),
                color: Color32::LIGHT_GRAY,
                ..Default::default()
            },
            len: span.0.len() + span.1.len() + span.2.len() + 1,
        },
    ))
}

fn cancelled_task(s: &str) -> IResult<&str, Style> {
    let mut inner = tuple((space0, tag("[x]"), not_line_ending, newline));
    let (extra, span) = inner(s)?;
    Ok((
        extra,
        Style {
            look: TextFormat {
                font_id: FontId::new(14.0, FontFamily::Proportional),
                color: Color32::DARK_RED,
                ..Default::default()
            },
            len: span.0.len() + span.1.len() + span.2.len() + 1,
        },
    ))
}

fn completed_task(s: &str) -> IResult<&str, Style> {
    let mut inner = tuple((space0, tag("[/]"), not_line_ending, newline));
    let (extra, span) = inner(s)?;
    Ok((
        extra,
        Style {
            look: TextFormat {
                font_id: FontId::new(14.0, FontFamily::Proportional),
                color: Color32::DARK_GREEN,
                ..Default::default()
            },
            len: span.0.len() + span.1.len() + span.2.len() + 1,
        },
    ))
}

fn code(s: &str) -> IResult<&str, Style> {
    let mut inner = delimited(tag("`"), take_until("`"), tag("`"));
    let (extra, span) = inner(s)?;
    Ok((
        extra,
        Style {
            look: TextFormat {
                font_id: FontId::new(14.0, FontFamily::Monospace),
                color: Color32::LIGHT_GRAY,
                background: Color32::DARK_GRAY,
                ..Default::default()
            },
            len: span.len() + 2,
        },
    ))
}

fn hyperlink(s: &str) -> IResult<&str, Style> {
    let allowed = |c: char| !c.is_whitespace() && c != '\n';
    let mut hyper = tuple((alphanumeric1, tag("://"), fold_many1(satisfy(allowed), || "".to_owned(), |mut x: String, i| {x.push(i); x})));
    let (extra, span) = hyper(s)?;
    Ok((
        extra,
        Style {
            look: TextFormat {
                font_id: FontId::new(14.0, FontFamily::Monospace),
                color: Color32::from_rgb(100, 168, 248),
                ..Default::default()
            },
            len: span.0.len() + span.1.len() + span.2.len(),
        },
    ))
}

fn style(s: &str) -> IResult<&str, Style> {
    let (extra, style) = alt((header, completed_task, cancelled_task, code, hyperlink))(s)?;
    Ok((extra, style))
}

fn parse(input: &str) -> IResult<&str, Vec<Style>> {
    let mut output = Vec::new();
    let mut current_input = input;

    while !current_input.is_empty() {
        let mut at_least_one_style = false;
        for (idx, _) in current_input.char_indices() {
            match style(&current_input[idx..]) {
                Ok((remaining, style)) => {
                    let text_until_style = &current_input[0..idx];
                    if !text_until_style.is_empty() {
                        output.push(Style {
                            look: TextFormat {
                                font_id: FontId::new(14.0, FontFamily::Proportional),
                                color: Color32::LIGHT_GRAY,
                                ..Default::default()
                            },
                            len: text_until_style.len(),
                        });
                    }
                    output.push(style);
                    current_input = remaining;
                    at_least_one_style = true;
                    break;
                }
                Err(nom::Err::Error(_)) => { /* no matches */ }
                Err(e) => return Err(e),
            }
        }

        if !at_least_one_style {
            output.push(Style {
                look: TextFormat {
                    font_id: FontId::new(14.0, FontFamily::Proportional),
                    color: Color32::LIGHT_GRAY,
                    ..Default::default()
                },
                len: current_input.len(),
            });
            break;
        }
    }

    Ok(("", output))
}

pub fn highlight(text: &str) -> LayoutJob {
    let mut job = LayoutJob::default();
    job.text = text.into();

    match parse(text) {
        Ok((_, styles)) => {
            let mut offset = 0;
            for style in styles {
                job.sections.push(LayoutSection {
                    byte_range: offset..offset + style.len,
                    leading_space: 0.0,
                    format: style.look,
                });
                offset += style.len;
            }
        }
        _ => {}
    }

    job
}

#[derive(Default, Debug)]
pub struct CachedLayoutJobBuilder {
    cached: Option<LayoutJob>
}

impl CachedLayoutJobBuilder {
    pub fn clear(&mut self) {
        self.cached = None
    }

    pub fn highlight(&mut self, text: &str) -> LayoutJob {
        if self.cached.is_none() {             
            self.cached = Some(highlight(text));
        }
        self.cached.to_owned().unwrap()
    }
}