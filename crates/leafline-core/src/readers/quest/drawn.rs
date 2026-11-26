//! The Quest Helper read without Windows OCR, for the website: the panel found by the buttons on
//! its header bar (as the minimap is found by its WORLD button), its rows by their pure black
//! text, objectives by the game's objective font or their counters' digit shapes, and only what
//! no font reads (titles, and targets with a letter the font hasn't shown yet) handed to the
//! caller's own text reader.
//!
//! Measured on 1,043 saved frames against the program's readings (Windows OCR), with ocrs as the
//! text reader: 4,787 objective lines read with the same counts; 126 more the program missed or
//! misread (checked by eye where it mattered); 55 the program read and this didn't; no count found
//! wrong. At UI Size Default Ratio (Filter Applied) the digits blur, so a count there stands only
//! where its digits' shapes and the row's text agree.

use image::Rgba;

use super::{
    CounterGlyphs, HEIGHT_IN_LINES, MAX_GAP, Objective, PAGE, PITCH, Quest, QuestReading, Row, WIDTH_IN_LINES,
    after_counter, assemble, clean_title, glyph_line, glyph_objective, has_text, is_text, on_white,
    past_counter_debris, settle_counters,
};
use crate::vision::glyphs::{ink_columns, ink_rows};
use crate::readers::ui;
use crate::vision::{Frame, Rect, color, preprocess};

/// Below this share of blue pixels a block is not one of the header's buttons; the white mark on
/// each takes up a little.
const BUTTON_FILL: f32 = 0.6;
/// How far the close button stands right of the minimise button, in button heights (26 x 20
/// window pixels at 2x).
const BUTTON_PITCH: std::ops::RangeInclusive<f32> = 1.15..=1.45;
/// Where the header's text starts, in button heights left of the minimise button: 380 pixels at
/// 2x, whether AUTO is on (orange) or off (grey).
const HEADER_START: f32 = 19.0;
/// How far off that the text's first ink may be, in button heights. Scenery can come within 0.5.
const HEADER_SLACK: f32 = 0.35;

/// The Quest Helper header's minimise button: a blue square with another (the close button) just
/// right of it, and no orange WORLD button after them as the minimap has. Largest first.
pub fn buttons(frame: &Frame) -> Vec<Rect> {
    let whole = Rect::new(0.0, 0.0, frame.width() as f32, frame.height() as f32);
    let min_height = frame.height() as f32 * 0.006;
    // 20 pixels at 2x in a 1440-pixel window; a blue patch of scenery can be any size.
    let max_height = frame.height() as f32 * 0.04;
    let squares: Vec<Rect> = color::find_blocks(frame, whole, 2, is_button_blue)
        .into_iter()
        .filter(|b| b.h >= min_height && b.h <= max_height && (0.8..=1.25).contains(&(b.w / b.h)))
        .filter(|b| share(frame, *b, is_button_blue) >= BUTTON_FILL)
        .collect();
    let mut found: Vec<Rect> = squares
        .iter()
        .filter(|b| {
            let h = b.h;
            let close = squares.iter().any(|c| {
                (c.y - b.y).abs() <= 0.15 * h && (c.h - h).abs() <= 0.15 * h && BUTTON_PITCH.contains(&((c.x - b.x) / h))
            });
            let world = Rect::from_edges(b.x + 2.3 * h, b.y + 0.2 * h, b.x + 3.3 * h, b.bottom() - 0.2 * h);
            close && share(frame, world, is_auto_orange) < 0.3
        })
        .copied()
        .collect();
    // An orange AUTO button (AUTO on) also marks the minimise button just right of it, for when
    // the blue squares run into blue scenery.
    let autos = color::find_blocks(frame, whole, 2, is_auto_orange)
        .into_iter()
        .filter(|b| b.h >= min_height && (1.6..=2.6).contains(&(b.w / b.h)))
        .filter(|b| share(frame, *b, is_auto_orange) >= 0.45)
        .map(|b| Rect::new(b.right() + 0.3 * b.h, b.y, b.h, b.h))
        .filter(|m| share(frame, Rect::from_edges(m.x + 0.15 * m.h, m.y + 0.15 * m.h, m.right() - 0.15 * m.h, m.bottom() - 0.15 * m.h), is_button_blue) >= 0.4);
    for minimise in autos {
        if !found.iter().any(|f| (f.x - minimise.x).abs() <= 0.5 * f.h && (f.y - minimise.y).abs() <= 0.5 * f.h) {
            found.push(minimise);
        }
    }
    found.sort_by(|a, b| (b.w * b.h).total_cmp(&(a.w * a.h)));
    found
}

/// The Quest Helper's header text (`Quest Helper (2/5)`), found by its buttons, in a picture
/// where the UI is drawn at 2x (the window itself, or the panel redrawn on the design grid). The
/// rectangle is the text's ink, as the overview's header line would be.
pub fn header(frame: &Frame) -> Option<Rect> {
    buttons(frame).into_iter().find_map(|button| header_at(frame, button))
}

/// The UI size the header's buttons are drawn at (10 design pixels tall), as one of the sizes the
/// game draws at: for frames with no HUD in view to tell.
pub fn scale(frame: &Frame, button: Rect) -> f32 {
    let measured = button.h / 10.0;
    let window = (frame.height() as f32 / ui::DESIGN_HEIGHT).max(1.0);
    [2.0, window, 1.0].into_iter().find(|s| (measured - s).abs() <= 0.08 * s).unwrap_or(measured)
}

fn header_at(frame: &Frame, button: Rect) -> Option<Rect> {
    let h = button.h;
    let start = button.x - HEADER_START * h;
    // The text's rows, from its words clear of the scenery that can come close to its start.
    let (top, bottom) = (button.y - 0.5 * h, button.bottom() + 0.5 * h);
    // Up to the AUTO button, whose border is as black as the text.
    let words = Rect::from_edges(start + HEADER_SLACK * h, top, button.x - 2.5 * h, bottom);
    let (top, bottom) = ink_rows(frame, words, is_text)?;
    if !(0.7 * h..=1.5 * h).contains(&((bottom - top) as f32)) {
        return None;
    }
    let line = Rect::from_edges(start - HEADER_SLACK * h, top as f32, words.right(), bottom as f32);
    let columns = ink_columns(frame, line, is_text);
    let left = columns.first()?.0;
    let right = columns.last()?.1;
    if left as f32 > start + HEADER_SLACK * h {
        return None;
    }
    Some(Rect::from_edges(left as f32, top as f32, right as f32, bottom as f32))
}

/// Where the whole panel is around a minimise button found at another UI size, with room to
/// spare: the part of the window to redraw on the design grid before [`header`] can place it.
pub fn around(button: Rect) -> Rect {
    let h = button.h;
    Rect::from_edges(button.x - (HEADER_START + 2.0) * h, button.y - 1.5 * h, button.x + 4.0 * h, button.bottom() + 28.0 * h)
}

/// The panel under a header found by [`header`], as [`super::panel`] places it.
pub fn panel(header: Rect) -> Rect {
    let h = header.h;
    Rect::from_edges(header.x - 0.8 * h, header.bottom(), header.x + WIDTH_IN_LINES * h, header.bottom() + HEIGHT_IN_LINES * h)
}

/// Reads the panel under `header` (its text's ink, from [`header`]); `digits` is the same picture
/// drawn to keep thin strokes, or `frame` itself. `read_text` reads the words in a rectangle of
/// `frame`, for titles and for targets with letters the objective font doesn't know.
pub fn read(
    frame: &Frame,
    digits: &Frame,
    header: Rect,
    page: Option<(u32, u32)>,
    smoothed: bool,
    read_text: &mut dyn FnMut(&Frame, Rect) -> Option<String>,
) -> QuestReading {
    let whole = panel(header);
    let h = header.h;
    // The panel's text, from just left of where every row starts: scenery dark enough to pass
    // for its text can stand right up to the panel's edge.
    let panel = Rect::from_edges(header.x - 0.25 * h, whole.y, whole.right(), whole.bottom());
    let pitch = PITCH * h;
    // Everything starts at the panel's text margin, just right of where the header's does.
    let margin = header.x..=header.x + 1.2 * h;
    let mut rows: Vec<Row> = Vec::new();
    let mut drawn = Vec::new();
    // For counters cut from a smoothed picture, what the row's text says they are: the digits'
    // shapes blur there (`20` read as `8`), so a count stands only where both agree.
    let mut checks: Vec<Option<Vec<(u32, u32)>>> = Vec::new();
    let mut previous = header.bottom();
    for (top, bottom) in text_rows(frame, panel, h) {
        let y = (top + bottom) as f32 / 2.0;
        if y - previous > MAX_GAP * pitch {
            break;
        }
        let band = Rect::from_edges(panel.x, top as f32, panel.right(), bottom as f32);
        let columns = ink_columns(frame, band, is_text);
        let Some(&(first, _)) = columns.first() else { continue };
        if !margin.contains(&(first as f32)) || !has_text(frame, panel, y, h) {
            continue;
        }
        previous = y;
        let line = glyph_line(frame, panel, y, h);
        let objective = line.as_deref().and_then(glyph_objective);
        // The title's own text, up to its close button at the panel's right.
        let text_area = || Rect::from_edges(first as f32 - 0.2 * h, y - 0.75 * h, panel.right() - 1.6 * h, y + 0.75 * h);
        if objective.is_none() && CounterGlyphs::read(frame, panel, y, h).is_none() {
            // Text that isn't an objective, on plain white, is a speech bubble's below the panel.
            if on_white(frame, panel, y, h) {
                continue;
            }
            let Some(text) = read_text(frame, text_area()) else { continue };
            // A counter whose digits couldn't be cut out (the UI drawn smoothed) still reads as
            // one in the text, as the program reads its own.
            if let Some((target, counters)) = super::objective(std::slice::from_ref(&text)) {
                // A count past its target (`28 / 20` items) is kept: only a clear slash lets the
                // text give one, and the quest's own lines say whether it can stand (kills stop at
                // the target; items held can pass it, when the target is read as the data has it).
                rows.push(Row::Objective { y, target: past_counter_debris(&target), counters });
                drawn.push(None);
                checks.push(None);
                continue;
            }
            let title = clean_title(&text);
            if title.len() < 3 {
                continue;
            }
            rows.push(Row::Title { y, title });
            drawn.push(None);
            checks.push(None);
            continue;
        }
        let target = match objective.as_ref().and_then(|(_, target)| target.clone()) {
            Some(target) => target,
            None => {
                // Only the words after the counter, which ocrs would otherwise run into them.
                let area = match CounterGlyphs::read(frame, panel, y, h) {
                    Some(counter) => Rect { x: counter.end as f32 + 0.2 * h, ..text_area() },
                    None => text_area(),
                };
                let area = Rect { w: (panel.right() - area.x).max(0.0), ..area };
                read_text(frame, area).map(|t| past_counter_debris(&after_counter(&t))).unwrap_or_default()
            }
        };
        let check = (smoothed && objective.is_none())
            .then(|| read_text(frame, text_area()).and_then(|t| super::objective(&[t])).map(|(_, c)| c).unwrap_or_default());
        rows.push(Row::Objective { y, target, counters: Vec::new() });
        drawn.push(objective.map(|(counter, _)| counter));
        checks.push(check);
    }
    let glyphs: Vec<Option<CounterGlyphs>> = rows
        .iter()
        .map(|row| {
            let plain = CounterGlyphs::read(frame, panel, row.y(), h)?;
            let inked = CounterGlyphs::read(digits, panel, row.y(), h).filter(|inked| inked.shaped().is_some());
            Some(inked.unwrap_or(plain))
        })
        .collect();
    let mut counters = settle_counters(&rows, &glyphs, &drawn);
    for (counter, check) in counters.iter_mut().zip(&checks) {
        if let (Some(read), Some(check)) = (*counter, check)
            && !check.contains(&read)
        {
            *counter = None;
        }
    }
    let mut quests: Vec<Quest> = assemble(&rows, &counters);
    // An objective whose target couldn't be read isn't one the tracker can follow.
    for quest in &mut quests {
        quest.objectives.retain(|o: &Objective| !o.target.is_empty());
    }
    quests.retain(|q| !q.title.is_empty() || !q.objectives.is_empty());
    if let Some((shown, _)) = page {
        quests.truncate(shown as usize);
    }
    QuestReading { quests, page, panel: whole }
}

/// The page shown in the header's text (`Quest Helper (2/5)`), as read.
pub fn page(text: &str) -> Option<(u32, u32)> {
    PAGE.captures(text).and_then(|c| Some((c[1].parse().ok()?, c[2].parse().ok()?)))
}

/// Runs of rows in `panel` holding the panel's pure black text, each at least half a line tall.
/// Rows a few pixels apart (a title's bold strokes thinned by scaling) stay one run.
fn text_rows(frame: &Frame, panel: Rect, text_height: f32) -> Vec<(u32, u32)> {
    let (x, y, w, h) = panel.to_pixels(frame.width(), frame.height());
    let inked = |py: u32| (x..x + w).any(|px| is_text(frame.get_pixel(px, py)));
    let join = (0.2 * text_height).max(1.0) as u32;
    let mut runs: Vec<(u32, u32)> = Vec::new();
    for py in y..y + h {
        if !inked(py) {
            continue;
        }
        match runs.last_mut() {
            Some(run) if py - run.1 <= join => run.1 = py + 1,
            _ => runs.push((py, py + 1)),
        }
    }
    runs.retain(|(top, bottom)| (bottom - top) as f32 >= 0.5 * text_height && ((bottom - top) as f32) <= 1.6 * text_height);
    runs
}

/// The share of `rect` that matches.
fn share(frame: &Frame, rect: Rect, matches: impl Fn(&Rgba<u8>) -> bool) -> f32 {
    let (x, y, w, h) = rect.to_pixels(frame.width(), frame.height());
    if w == 0 || h == 0 {
        return 0.0;
    }
    let hits = (y..y + h).flat_map(|py| (x..x + w).map(move |px| (px, py))).filter(|&(px, py)| matches(frame.get_pixel(px, py))).count();
    hits as f32 / (w * h) as f32
}

/// The AUTO button's orange (and the minimap's WORLD button), from (221, 85, 17) at its top to (255, 136, 85) at its foot, and as a
/// browser's share weakens it ((213, 126, 77), (232, 145, 106)).
fn is_auto_orange(px: &Rgba<u8>) -> bool {
    let [r, g, b, _] = px.0.map(i16::from);
    r >= 160 && r - g >= 50 && r - b >= 80 && g >= b - 10
}

/// The header's minimise and close buttons, from (17, 153, 221) to (85, 204, 255), and as a
/// browser's share weakens them ((108, 197, 234)). Far greener than a sky's (119, 162, 209).
fn is_button_blue(px: &Rgba<u8>) -> bool {
    let [r, g, b, _] = px.0.map(i16::from);
    b >= 170 && b - r >= 100 && g - r >= 70 && preprocess::luminance(px) >= 80
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    /// A made-up Quest Helper header at 2x: a light bar, black text where the header's text starts,
    /// an orange AUTO button and the two blue buttons, as measured on the game's.
    fn header_bar(auto: [u8; 3]) -> Frame {
        let mut frame = Frame::from_pixel(700, 600, Rgba([90, 140, 200, 255]));
        let fill = |frame: &mut Frame, x0: u32, y0: u32, w: u32, h: u32, c: [u8; 3]| {
            for y in y0..y0 + h {
                for x in x0..x0 + w {
                    frame.put_pixel(x, y, Rgba([c[0], c[1], c[2], 255]));
                }
            }
        };
        fill(&mut frame, 100, 40, 460, 34, [204, 216, 227]);
        // The text: a run of letters 20 pixels tall, starting 380 pixels left of the minimise button.
        for i in 0..16 {
            fill(&mut frame, 124 + 11 * i, 48, 6, 20, [0, 0, 0]);
        }
        fill(&mut frame, 460, 46, 38, 20, auto);
        fill(&mut frame, 504, 46, 20, 20, [34, 170, 238]);
        fill(&mut frame, 530, 46, 20, 20, [34, 170, 238]);
        frame
    }

    #[test]
    fn the_header_is_found_by_its_buttons() {
        for auto in [[238, 119, 51], [187, 187, 187]] {
            let frame = header_bar(auto);
            assert_eq!(buttons(&frame), [Rect::new(504.0, 46.0, 20.0, 20.0)], "AUTO {auto:?}");
            let header = header(&frame).expect("the header");
            assert_eq!((header.x, header.y, header.h), (124.0, 48.0, 20.0));
            assert!((scale(&frame, buttons(&frame)[0]) - 2.0).abs() < 1e-6);
        }
    }

    #[test]
    fn the_minimaps_buttons_are_not_the_headers() {
        // Two blue squares with the orange WORLD button after them.
        let mut frame = header_bar([187, 187, 187]);
        for y in 46..66 {
            for x in 556..596 {
                frame.put_pixel(x, y, Rgba([221, 153, 17, 255]));
            }
        }
        assert!(buttons(&frame).is_empty());
    }

    #[test]
    fn reads_the_page() {
        assert_eq!(page("Quest Helper (2/5)"), Some((2, 5)));
        assert_eq!(page("Quest Helper"), None);
    }
}
