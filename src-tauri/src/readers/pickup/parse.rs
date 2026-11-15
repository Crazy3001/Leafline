//! One Pickup Log line, from the text OCR read off it.
//!
//! The game words each gain the same way every time, so a line is told by its fixed words:
//! `You received EXP (+21)`, `Bonus EXP for PARTY (+5)`, `You have gained mesos (+27)` and
//! `Leaf x1 earned. (Etc)`. OCR slips a letter or two of those words, so they are compared by
//! their letters with a little give, and a line is never moved from one kind to another by it.
//! Anything else (a name tag or a speech bubble passing behind the box) is no line.

use std::sync::LazyLock;

use regex::Regex;

use crate::readers::inventory::Tab;

/// A gain, as one line of the log words it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Gain {
    /// `You received EXP (+n)`. Whose it is comes from the line's colour, not its words.
    Exp { amount: u64 },
    /// `Bonus EXP for PARTY (+n)`.
    PartyBonus { amount: u64 },
    /// `You have gained mesos (+n)`.
    Mesos { amount: u64 },
    /// `<name> x<quantity> earned. (<tab>)`. A long name is cut short with `...`, which is left
    /// out of `name`.
    Item { name: String, quantity: u32, tab: Option<Tab>, cut_short: bool },
}

/// A line read once.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parsed {
    pub gain: Gain,
    /// The figure came with its `+`. Without it, `(+43)` may have been read as `(443)`: the
    /// figure is then taken with its first digit dropped when that digit is a 4, the way OCR
    /// reads a `+` most often, so a misread can only count less, never more.
    pub sure: bool,
}

impl Parsed {
    /// The figures a vote compares: the kind, amount and quantity, not an item's name.
    pub fn figures(&self) -> (u8, u64) {
        match &self.gain {
            Gain::Exp { amount } => (0, *amount),
            Gain::PartyBonus { amount } => (1, *amount),
            Gain::Mesos { amount } => (2, *amount),
            Gain::Item { quantity, tab, .. } => (3 + tab.map_or(0, |t| t as u8 + 1), u64::from(*quantity)),
        }
    }
}

/// The last bracketed part of a line: `(+27)`, `(Etc)`. OCR reads a bracket as `{`, `[`, `|`,
/// `l` or `1` now and then, so the closing one may be missing.
static LAST_BRACKET: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[(\[{]([^()\[\]{}]*)[)\]}]?\s*$").unwrap());
/// An item's quantity, as the last `x<digits>` before `earned`: digits in an item's own name
/// never count. OCR reads the `x` as `×` or `X`, and a 1 as `l`, `I` or `|`.
static QUANTITY: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)^(.*?)\s*[x×]\s*([0-9lI|]{1,4})\s*$").unwrap());
/// A figure whose opening bracket OCR lost: `'+19)`.
static BARE_FIGURE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\+\s*([0-9OoIl|,.]+)\s*[)\]}]\s*$").unwrap());
/// A count OCR misread so that it can't be told (`YI` for `x1`): a short last word with no
/// lower-case letters but `l` or `x`.
static LOST_COUNT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(.*\S)\s+[^a-km-wyz\s]{1,4}$").unwrap());
/// A tab whose opening bracket OCR lost or read as a letter: `OJse)`.
static BARE_TAB: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s(\S{2,7})[)\]}]\s*$").unwrap());

/// The lines whose words are always the same.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Fixed {
    Exp,
    PartyBonus,
    Mesos,
}

/// The line `text` reads as, or `None` when it is no Pickup Log line or its figure can't be read.
pub fn parse(text: &str) -> Option<Parsed> {
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let (before, inside) = split_bracket(&text)?;
    if let Some(item) = item(before, inside) {
        return Some(item);
    }
    let (amount, sure) = amount(inside)?;
    let gain = match fixed(before)? {
        Fixed::Exp => Gain::Exp { amount },
        Fixed::PartyBonus => Gain::PartyBonus { amount },
        Fixed::Mesos => Gain::Mesos { amount },
    };
    Some(Parsed { gain, sure })
}

/// Which fixed line the words before the figure are. OCR often drops a word or two of a pixel
/// font, so a line is also told by the words of it that are left, each of which only one kind of
/// line has: `have`, `gained` and `mesos`; `received`; `Bonus` and `PARTY`.
fn fixed(before: &str) -> Option<Fixed> {
    let whole = letters(before);
    if is_like(&whole, "youreceivedexp") {
        return Some(Fixed::Exp);
    }
    if is_like(&whole, "bonusexpforparty") {
        return Some(Fixed::PartyBonus);
    }
    if is_like(&whole, "youhavegainedmesos") {
        return Some(Fixed::Mesos);
    }
    let words: Vec<String> = before.split_whitespace().map(letters).filter(|w| !w.is_empty()).collect();
    let has = |word: &str, slips: usize| words.iter().any(|w| w.len() + 1 >= word.len() && strsim::levenshtein(w, word) <= slips);
    let bonus = has("bonus", 1) || has("party", 1);
    let exp = has("received", 2);
    let mesos = has("mesos", 1) || has("gained", 1) || has("have", 1);
    match (bonus, exp, mesos) {
        (true, false, false) => Some(Fixed::PartyBonus),
        (false, true, false) => Some(Fixed::Exp),
        (false, false, true) => Some(Fixed::Mesos),
        _ => None,
    }
}

/// The text before the last bracket and what the bracket holds.
fn split_bracket(text: &str) -> Option<(&str, &str)> {
    if let Some(found) = LAST_BRACKET.captures(text) {
        let whole = found.get(0)?;
        return Some((text[..whole.start()].trim(), found.get(1)?.as_str().trim()));
    }
    let Some(found) = BARE_FIGURE.captures(text) else {
        let found = BARE_TAB.captures(text)?;
        let whole = found.get(1)?;
        return Some((text[..whole.start()].trim(), whole.as_str()));
    };
    let whole = found.get(0)?;
    // From the `+` to the last digit.
    let figure = &text[whole.start()..found.get(1)?.end()];
    Some((text[..whole.start()].trim_end_matches(['\'', '`', '|']).trim(), figure))
}

/// `<name> x<quantity> earned.` and the tab in the bracket after it.
fn item(before: &str, inside: &str) -> Option<Parsed> {
    // `earned.` is the line's last word before the bracket; OCR can join it to the quantity.
    // `earned.` is the last word, a letter or two off: `eamed`, `earneq`.
    let last = before.split_whitespace().last()?;
    let word = letters(last);
    if word.len() < 4 || strsim::levenshtein(&word, "earned") > 2 || !last.chars().next().is_some_and(char::is_alphabetic) {
        return None;
    }
    let head = before[..before.len() - last.len()].trim();
    // A count that can't be read is taken as one, the fewest it can be; such a read is outvoted
    // by any that read the count.
    let (mut name, quantity, sure) = match QUANTITY.captures(head) {
        Some(found) => {
            let digits: String = found[2].chars().map(|c| if c.is_ascii_digit() { c } else { '1' }).collect();
            (found[1].trim().to_string(), digits.parse().ok().filter(|&q| q > 0)?, true)
        }
        None => (LOST_COUNT.captures(head).map_or(head, |found| found.get(1).map_or(head, |m| m.as_str())).to_string(), 1, false),
    };
    // A name cut short ends in `...`, which OCR may read as two or three marks of any kind.
    let trimmed = name.trim_end_matches(['.', ',', '·', '…', ':', ';']);
    let cut_short = name.len() - trimmed.len() >= 2 || name.ends_with('…');
    name = trimmed.trim().to_string();
    if letters(&name).len() < 2 {
        return None;
    }
    Some(Parsed { gain: Gain::Item { name, quantity, tab: tab(inside), cut_short }, sure })
}

/// The inventory tab a bracket names, read with a letter or so wrong.
fn tab(inside: &str) -> Option<Tab> {
    let read = letters(inside);
    let tabs = [("equip", Tab::Equip), ("use", Tab::Use), ("setup", Tab::Setup), ("etc", Tab::Etc), ("cash", Tab::Cash)];
    tabs.iter()
        .map(|(word, tab)| (strsim::levenshtein(&read, word), *tab, word.len()))
        .filter(|(wrong, _, len)| *wrong == 0 || (*len >= 4 && *wrong <= 1))
        .min_by_key(|(wrong, _, _)| *wrong)
        .map(|(_, tab, _)| tab)
}

/// The amount in `(+27)`, and whether its `+` was read.
fn amount(inside: &str) -> Option<(u64, bool)> {
    let inside: String = inside.chars().filter(|c| !c.is_whitespace() && *c != ',' && *c != '.').collect();
    let (sure, digits) = match inside.strip_prefix('+') {
        Some(rest) => (true, rest.to_string()),
        // A `+` read as a `t` or `f` is still a `+` that was there.
        None => match inside.strip_prefix(['t', 'f', 'T']) {
            Some(rest) => (true, rest.to_string()),
            None => (false, inside),
        },
    };
    let digits: String = digits
        .chars()
        .map(|c| match c {
            'O' | 'o' | 'D' => '0',
            'l' | 'I' | '|' | 'i' => '1',
            // The font's 9 has a tail OCR takes for a `g` or `q`.
            'g' | 'q' => '9',
            other => other,
        })
        .collect();
    if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) || digits.len() > 9 {
        return None;
    }
    if sure {
        return Some((digits.parse().ok()?, true));
    }
    // `(443)` for `(+43)`: the 4 is most likely the `+`.
    let digits = match digits.strip_prefix('4') {
        Some(rest) if !rest.is_empty() => rest,
        _ => digits.as_str(),
    };
    Some((digits.parse().ok()?, false))
}

/// The letters of `text`, lower case: what OCR gets right of a word when it slips on a glyph.
fn letters(text: &str) -> String {
    text.chars().filter(char::is_ascii_alphabetic).map(|c| c.to_ascii_lowercase()).collect()
}

/// Whether `read` is the fixed words `fixed`, a letter or two off: one slip per eight letters.
fn is_like(read: &str, fixed: &str) -> bool {
    read.len() >= fixed.len() / 2 && strsim::levenshtein(read, fixed) <= fixed.len().div_ceil(8)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gain(text: &str) -> Option<Gain> {
        parse(text).map(|p| p.gain)
    }

    #[test]
    fn reads_each_kind_of_line() {
        assert_eq!(gain("You received EXP (+21)"), Some(Gain::Exp { amount: 21 }));
        assert_eq!(gain("Bonus EXP for PARTY (+5)"), Some(Gain::PartyBonus { amount: 5 }));
        assert_eq!(gain("You have gained mesos (+27)"), Some(Gain::Mesos { amount: 27 }));
        assert_eq!(
            gain("Leaf x1 earned. (Etc)"),
            Some(Gain::Item { name: "Leaf".into(), quantity: 1, tab: Some(Tab::Etc), cut_short: false })
        );
        assert_eq!(
            gain("Red Potion x12 earned. (Use)"),
            Some(Gain::Item { name: "Red Potion".into(), quantity: 12, tab: Some(Tab::Use), cut_short: false })
        );
    }

    #[test]
    fn a_letter_or_two_wrong_in_the_fixed_words_still_reads() {
        assert_eq!(gain("You have galned mesos (+1,204)"), Some(Gain::Mesos { amount: 1204 }));
        assert_eq!(gain("Y0u receivcd EXP (+9)"), Some(Gain::Exp { amount: 9 }));
        assert_eq!(gain("Snail Shell xl eamed. {Etc)").map(|g| matches!(g, Gain::Item { quantity: 1, .. })), Some(true));
    }

    #[test]
    fn other_text_is_no_line() {
        assert_eq!(gain("Swiftleaf: anyone selling mesos?"), None);
        assert_eq!(gain("You received EXP"), None, "no figure");
        assert_eq!(gain("You received EXP (+2a)"), None, "a figure that isn't one");
        assert_eq!(gain("Leaf x1 earned. (Etc) Leaf"), None);
    }

    #[test]
    fn an_amount_without_its_plus_can_only_count_less() {
        let read = parse("You have gained mesos (443)").unwrap();
        assert_eq!((read.gain, read.sure), (Gain::Mesos { amount: 43 }, false));
        let read = parse("You have gained mesos (27)").unwrap();
        assert_eq!((read.gain, read.sure), (Gain::Mesos { amount: 27 }, false));
        assert!(parse("You have gained mesos (+43)").unwrap().sure);
    }

    #[test]
    fn words_ocr_dropped_leave_enough_to_tell_the_line() {
        assert_eq!(gain("You have (+38)"), Some(Gain::Mesos { amount: 38 }));
        assert_eq!(gain("gained mesos (+24)"), Some(Gain::Mesos { amount: 24 }));
        assert_eq!(gain("received EXP '+19)"), Some(Gain::Exp { amount: 19 }));
        assert_eq!(gain("EXP (+35)"), None, "EXP alone could be the party bonus");
        assert_eq!(gain("(+24)"), None);
    }

    #[test]
    fn misreads_seen_in_the_recording_still_read() {
        assert_eq!(gain("You have qained mesos (+g)"), Some(Gain::Mesos { amount: 9 }));
        assert_eq!(gain("You have qained mesos {+.3g)"), Some(Gain::Mesos { amount: 39 }));
        assert!(matches!(gain("Red Potion earneq. OJse)"), Some(Gain::Item { ref name, quantity: 1, .. }) if name == "Red Potion"));
        assert!(matches!(gain("Red Potion V.I aqrned. 0.-Jse)"), Some(Gain::Item { ref name, .. }) if name == "Red Potion"));
        assert!(matches!(gain("Red Potion•xl earned. OJse)"), Some(Gain::Item { quantity: 1, .. })));
    }

    #[test]
    fn a_count_that_cant_be_read_is_one_and_unsure() {
        let read = parse("Stiff Feather YI earned. {Etc)").unwrap();
        assert_eq!(read.gain, Gain::Item { name: "Stiff Feather".into(), quantity: 1, tab: Some(Tab::Etc), cut_short: false });
        assert!(!read.sure);
    }

    #[test]
    fn digits_in_a_name_are_not_its_quantity() {
        assert_eq!(
            gain("Arrow for Bow 100 x3 earned. (Use)"),
            Some(Gain::Item { name: "Arrow for Bow 100".into(), quantity: 3, tab: Some(Tab::Use), cut_short: false })
        );
    }

    #[test]
    fn a_name_cut_short_is_marked() {
        assert_eq!(
            gain("Scroll for One-Handed Sw... x1 earned. (Use)"),
            Some(Gain::Item { name: "Scroll for One-Handed Sw".into(), quantity: 1, tab: Some(Tab::Use), cut_short: true })
        );
    }

    #[test]
    fn a_tab_read_wrong_is_still_found_or_left_unknown() {
        assert_eq!(tab("Etc"), Some(Tab::Etc));
        assert_eq!(tab("Equlp"), Some(Tab::Equip));
        assert_eq!(tab("Set-up"), Some(Tab::Setup));
        assert_eq!(tab("Uso"), None, "too short to guess");
    }

    #[test]
    fn votes_compare_figures_not_names() {
        let a = parse("Leaf x1 earned. (Etc)").unwrap();
        let b = parse("Lcaf x1 earned. (Etc)").unwrap();
        assert_eq!(a.figures(), b.figures());
        assert_ne!(a.figures(), parse("Leaf x2 earned. (Etc)").unwrap().figures());
        assert_ne!(parse("You received EXP (+5)").unwrap().figures(), parse("Bonus EXP for PARTY (+5)").unwrap().figures());
    }
}
