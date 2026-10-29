//! Grouping OCR words into lines.
//!
//! Engines tend to merge words that share a baseline into one line even when they belong to
//! different panels (two tooltips side by side, a tooltip over chat). Rebuilding lines from
//! words with a tight horizontal gap keeps separate panels apart.

use serde::Serialize;

use super::Rect;

#[derive(Debug, Clone, Serialize)]
pub struct Word {
    pub text: String,
    pub rect: Rect,
}

#[derive(Debug, Clone, Serialize)]
pub struct Line {
    pub text: String,
    pub rect: Rect,
    pub words: Vec<Word>,
}

impl Line {
    fn new(word: Word) -> Self {
        Self { text: word.text.clone(), rect: word.rect, words: vec![word] }
    }

    fn push(&mut self, word: Word) {
        self.text.push(' ');
        self.text.push_str(&word.text);
        self.rect = self.rect.union(&word.rect);
        self.words.push(word);
    }

    /// Horizontal gap from the end of this line to `word`, if `word` sits on the same row.
    fn gap_to(&self, word: &Word) -> Option<f32> {
        let last = self.words.last()?.rect;
        let min_height = last.h.min(word.rect.h);
        let same_row = last.vertical_overlap(&word.rect) >= 0.5 * min_height;
        let gap = word.rect.x - last.right();
        let max_gap = 1.5 * last.h.max(word.rect.h);
        (same_row && gap >= -0.5 * min_height && gap <= max_gap).then_some(gap)
    }
}

/// Groups words into lines, ordered top to bottom then left to right.
pub fn group_lines(words: &[Word]) -> Vec<Line> {
    let mut sorted: Vec<&Word> = words.iter().collect();
    sorted.sort_by(|a, b| a.rect.x.total_cmp(&b.rect.x));

    let mut lines: Vec<Line> = Vec::new();
    for word in sorted {
        let best = lines
            .iter_mut()
            .filter_map(|line| line.gap_to(word).map(|gap| (gap, line)))
            .min_by(|a, b| a.0.total_cmp(&b.0));
        match best {
            Some((_, line)) => line.push(word.clone()),
            None => lines.push(Line::new(word.clone())),
        }
    }

    lines.sort_by(|a, b| a.rect.y.total_cmp(&b.rect.y).then(a.rect.x.total_cmp(&b.rect.x)));
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    fn word(text: &str, x: f32, y: f32) -> Word {
        Word { text: text.into(), rect: Rect::new(x, y, 10.0 * text.len() as f32, 12.0) }
    }

    #[test]
    fn joins_adjacent_words_and_splits_distant_columns() {
        let words = [
            word("Type:", 0.0, 100.0),
            word("Hat", 55.0, 101.0),
            word("Type:", 300.0, 100.0),
            word("Hat", 355.0, 100.0),
            word("Brown", 0.0, 50.0),
        ];
        let lines = group_lines(&words);
        let texts: Vec<&str> = lines.iter().map(|l| l.text.as_str()).collect();
        assert_eq!(texts, ["Brown", "Type: Hat", "Type: Hat"]);
        assert_eq!(lines[1].rect.x, 0.0);
        assert_eq!(lines[2].rect.x, 300.0);
    }
}
