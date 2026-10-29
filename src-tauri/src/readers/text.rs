//! Helpers for cleaning up OCR text.

/// Replaces letters OCR commonly confuses with digits, for use on number-only fields.
pub fn digitize(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'O' | 'o' | 'D' | 'Q' => '0',
            'I' | 'l' | '|' | 'i' | '!' => '1',
            'Z' | 'z' => '2',
            'S' | 's' => '5',
            'B' => '8',
            other => other,
        })
        .collect()
}

/// Parses a number that may contain thousands separators ("38,200", "38.200", "36 644").
pub fn parse_grouped(s: &str) -> Option<u64> {
    let digits: String = digitize(s).chars().filter(char::is_ascii_digit).collect();
    if digits.is_empty() { None } else { digits.parse().ok() }
}

/// Lowercase letters and digits only, for fuzzy comparisons.
pub fn squash(s: &str) -> String {
    s.chars().filter(|c| c.is_alphanumeric()).flat_map(char::to_lowercase).collect()
}

/// Similarity in `0.0..=1.0` between two strings after squashing.
pub fn similarity(a: &str, b: &str) -> f64 {
    strsim::normalized_levenshtein(&squash(a), &squash(b))
}

/// The entry of `options` most similar to `s`, if it clears `threshold`.
pub fn best_match<'a>(s: &str, options: &[&'a str], threshold: f64) -> Option<&'a str> {
    options
        .iter()
        .map(|&option| (similarity(s, option), option))
        .max_by(|a, b| a.0.total_cmp(&b.0))
        .filter(|(score, _)| *score >= threshold)
        .map(|(_, option)| option)
}

/// Whether the first word of `text` reads like `word` (for example `Remaning` and `Remaining`).
pub fn starts_like(text: &str, word: &str) -> bool {
    let first = text.split([':', ' ']).next().unwrap_or_default();
    squash(first).len() >= 3 && similarity(first, word) >= 0.7
}

/// Removes bullets and stray punctuation OCR picks up at the start and end of a line.
pub fn trim_noise(s: &str) -> &str {
    s.trim_matches(|c: char| !c.is_alphanumeric() && !"()+-%".contains(c))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grouped_numbers() {
        assert_eq!(parse_grouped("38,200"), Some(38200));
        assert_eq!(parse_grouped("36 644"), Some(36644));
        assert_eq!(parse_grouped("1O,OOO"), Some(10000));
        assert_eq!(parse_grouped("--"), None);
    }

    #[test]
    fn fuzzy_matching() {
        assert_eq!(best_match("Weapon Def", &["Weapon Def.", "Magic Def."], 0.7), Some("Weapon Def."));
        assert_eq!(best_match("Banana", &["Weapon Def.", "Magic Def."], 0.7), None);
    }

    #[test]
    fn trims_bullets() {
        assert_eq!(trim_noise("• «Remaining Enhancements: 7—"), "Remaining Enhancements: 7");
        assert_eq!(trim_noise("·MaxHP: +10"), "MaxHP: +10");
    }
}
