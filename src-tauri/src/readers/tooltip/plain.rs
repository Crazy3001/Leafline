//! Tooltips of items with no stats, such as quest and Etc items: a name, perhaps a small orange
//! tag, the icon in a pale box, and a description.
//!
//! These have no separator or job flags to find them by, and the panel is see-through, so its
//! colour changes with whatever is behind it. A tooltip's name has a solid blue bullet before it,
//! and the icon box below it. The box is see-through too and often shows only in pieces, and
//! other see-through panels can share its shade, so it takes both to make a tooltip.

use image::Rgba;

use super::parse::ItemTooltip;
use super::{LINE_PITCH, Panel, TARGET_TEXT_HEIGHT, parse};
use crate::error::Result;
use crate::readers::text::{best_match, trim_noise};
use crate::vision::{Frame, Line, Ocr, Rect, group_lines, preprocess};

/// Offsets in game pixels, the same scale as the rest of the tooltip.
/// The icon box's width.
const ICON_BOX: f32 = 68.0;
/// How tall a name's text is, which gives the scale.
const NAME_HEIGHT: f32 = 10.0;
/// How far below the name the box is looked for: past a tag, and well into the box.
const BOX_SEARCH: f32 = 100.0;
/// How much of the box's shade counts as a box, in square game pixels. A fifth of the box is
/// enough: it shows in pieces over scenery.
const MIN_BOX_AREA: f32 = 125.0;
/// The bullet is 13 square game pixels. OCR may read it as part of the line or leave it out, so
/// this much space is searched left of the line's start, and some of the line.
const MIN_BULLET_AREA: f32 = 6.0;
const BULLET_SEARCH: f32 = 14.0;
/// The name and any tags sit in this much space below the top of the name.
const HEADER_HEIGHT: f32 = 46.0;
/// How wide the panel's text can run.
const TEXT_WIDTH: f32 = 290.0;
/// Lines of one wrapped name are closer than this; a tag sits further down.
const NAME_WRAP: f32 = 16.0;

/// Tags as printed in game; a reading close to one takes its spelling.
const TAGS: [&str; 3] = ["Quest Item", "Untradable", "One-of-a-kind"];

/// The icon box's shade, lighter or darker with what is behind it.
fn is_icon_box(px: &Rgba<u8>) -> bool {
    let [r, g, b, _] = px.0.map(i16::from);
    (150..=225).contains(&r) && (r - g).abs() <= 3 && (10..=20).contains(&(b - r))
}

/// The bullet before a tooltip's name, solid (85, 204, 255) and blended only when scaled.
fn is_bullet(px: &Rgba<u8>) -> bool {
    let [r, g, b, _] = px.0.map(i16::from);
    (r - 85).abs() <= 16 && (g - 204).abs() <= 16 && (b - 255).abs() <= 16
}

fn count(frame: &Frame, area: Rect, matches: fn(&Rgba<u8>) -> bool) -> f32 {
    let (x, y, w, h) = area.to_pixels(frame.width(), frame.height());
    (y..y + h).map(|row| (x..x + w).filter(|&col| matches(frame.get_pixel(col, row))).count()).sum::<usize>() as f32
}

/// Whether `line` has a bullet before it and an icon box under it.
fn is_name(frame: &Frame, line: &Line) -> bool {
    let unit = line.rect.h / NAME_HEIGHT;
    let area = unit * unit;
    let (x, y) = (line.rect.x, line.rect.y);
    let beside = Rect::new(x - BULLET_SEARCH * unit, y - 2.0 * unit, (BULLET_SEARCH + 6.0) * unit, 14.0 * unit);
    let below = Rect::new(x - 8.0 * unit, line.rect.bottom(), (ICON_BOX + 8.0) * unit, BOX_SEARCH * unit);
    count(frame, beside, is_bullet) >= MIN_BULLET_AREA * area && count(frame, below, is_icon_box) >= MIN_BOX_AREA * area
}

/// Reads the plain tooltips in `frame`, leaving alone any line inside `panels`, which were found
/// as equipment.
pub fn read_all(frame: &Frame, overview: &[Line], panels: &[Panel], ocr: &Ocr) -> Result<Vec<ItemTooltip>> {
    let inside = |line: &Line| {
        panels.iter().any(|p| {
            let (x, y) = (line.rect.x, line.rect.center_y());
            x >= p.rect.x && x <= p.rect.right() && y >= p.rect.y && y <= p.rect.bottom()
        })
    };
    let mut names: Vec<&Line> = overview.iter().filter(|l| !inside(l) && is_name(frame, l)).collect();
    names.sort_by(|a, b| a.rect.y.total_cmp(&b.rect.y));

    let mut items: Vec<ItemTooltip> = Vec::new();
    for line in names {
        // Further lines over the same box belong to the tooltip already read.
        if items.iter().any(|item| item.panel.vertical_overlap(&line.rect) > 0.0) {
            continue;
        }
        items.push(read(frame, line, ocr)?);
    }
    Ok(items)
}

fn read(frame: &Frame, name: &Line, ocr: &Ocr) -> Result<ItemTooltip> {
    let unit = name.rect.h / NAME_HEIGHT;
    let scale = (TARGET_TEXT_HEIGHT / (LINE_PITCH * unit)).clamp(1.5, 4.0);
    let panel = Rect::new(name.rect.x - 2.0 * unit, name.rect.y - 4.0 * unit, TEXT_WIDTH * unit, HEADER_HEIGHT * unit);
    let words = ocr.words_in(frame, panel, scale, |crop| preprocess::upscale(crop, scale))?;
    let (text, tags) = name_and_tags(&group_lines(&words), unit);
    let (name, upgrades) = parse::split_name(&text);
    let item_type = (!tags.is_empty()).then(|| tags.join(" · "));
    Ok(ItemTooltip { name, upgrades, item_type, panel, plain: true, ..ItemTooltip::default() })
}

/// The name, joined if it wrapped, and the tags below it, in their printed spelling.
fn name_and_tags(lines: &[Line], unit: f32) -> (String, Vec<String>) {
    let mut lines: Vec<&Line> = lines.iter().filter(|l| !trim_noise(&l.text).is_empty()).collect();
    lines.sort_by(|a, b| a.rect.y.total_cmp(&b.rect.y));
    let (mut name, mut tags): (Vec<&str>, Vec<String>) = (Vec::new(), Vec::new());
    let mut previous: Option<f32> = None;
    for line in lines {
        let text = trim_noise(&line.text);
        let wraps = name.is_empty() || previous.is_some_and(|y| line.rect.y - y < NAME_WRAP * unit);
        if wraps {
            name.push(text);
            previous = Some(line.rect.y);
        } else {
            // With no tag the description starts here, so only a known tag counts. Orange on
            // purple is the hardest text in the tooltip: `Quest` has come back `Clijest`.
            tags.extend(best_match(text, &TAGS, 0.6).map(str::to_string));
            previous = None;
        }
    }
    (name.join(" "), tags)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vision::Word;

    fn line(text: &str, y: f32) -> Line {
        let rect = Rect::new(2000.0, y, 200.0, 20.0);
        Line { text: text.into(), rect, words: vec![Word { text: text.into(), rect }] }
    }

    #[test]
    fn a_name_comes_apart_from_its_tag() {
        for tag in ["auest Item", "Clijest Item", "CllJest Item"] {
            let lines = [line("• Sera's Mirror", 1050.0), line(tag, 1092.0)];
            assert_eq!(name_and_tags(&lines, 2.0), ("Sera's Mirror".to_string(), vec!["Quest Item".to_string()]));
        }
    }

    #[test]
    fn a_description_is_not_a_tag() {
        let lines = [
            line("• Earring LUK Scroll: Intermediate", 806.0),
            line("Improves Luck on earrings.", 856.0),
            line("Success rate: 60%, LUK +2", 888.0),
        ];
        assert_eq!(name_and_tags(&lines, 2.0), ("Earring LUK Scroll: Intermediate".to_string(), vec![]));
    }

    #[test]
    fn a_wrapped_name_is_joined_and_a_tag_is_optional() {
        let lines = [line("• Scroll for Two-Handed Sword", 1050.0), line("for Attack 60%", 1078.0)];
        assert_eq!(name_and_tags(&lines, 2.0), ("Scroll for Two-Handed Sword for Attack 60%".to_string(), vec![]));
    }

    /// A frame of `background` with a 13 by 4 bullet at `(100, 100)` if `bullet`, and a block of
    /// the box's shade under it if `icon_box`.
    fn scene(background: [u8; 3], bullet: bool, icon_box: bool) -> Frame {
        Frame::from_fn(400, 400, |x, y| {
            let [r, g, b] = background;
            if bullet && (100..113).contains(&x) && (100..104).contains(&y) {
                Rgba([85, 204, 255, 255])
            } else if icon_box && (100..200).contains(&x) && (130..200).contains(&y) {
                Rgba([204, 204, 219, 255])
            } else {
                Rgba([r, g, b, 255])
            }
        })
    }

    #[test]
    fn a_name_needs_both_its_bullet_and_the_box() {
        let mut name = line("Sera's Mirror", 96.0);
        name.rect = Rect::new(114.0, 96.0, 200.0, 20.0);
        const PANEL: [u8; 3] = [74, 76, 120];
        assert!(is_name(&scene(PANEL, true, true), &name));
        // The Quest Helper has the box's shade but no bullet.
        assert!(!is_name(&scene(PANEL, false, true), &name));
        assert!(!is_name(&scene(PANEL, true, false), &name));
    }

    #[test]
    fn the_box_shade_is_pale_and_slightly_blue() {
        for shade in [[204, 204, 219], [192, 193, 208], [185, 185, 199]] {
            assert!(is_icon_box(&Rgba([shade[0], shade[1], shade[2], 255])));
        }
        // The panel, an empty inventory slot, and the white of the name.
        for other in [[74, 76, 120], [224, 224, 218], [255, 255, 255]] {
            assert!(!is_icon_box(&Rgba([other[0], other[1], other[2], 255])));
        }
    }
}
