//! Putting the game's quest log together from what the Quest window shows. Only the rows in view
//! can be read, so each tab is gathered from the views the player scrolls through, and a tab
//! counts as seen whole once one run of rows reaches from its top to its end with every region
//! holding as many quests as its bar says.

use crate::readers::questlog::{LogItem, LogQuest, Scroll};
use crate::readers::text::squash;

/// Two readings of a row this alike are the same row: OCR can read it a letter or two apart.
const SAME_ROW: f64 = 0.85;

/// A run of rows read one after another, and whether it starts at the list's top or reaches its
/// end.
#[derive(Debug, Clone, Default, PartialEq)]
struct Piece {
    items: Vec<LogItem>,
    top: bool,
    end: bool,
}

/// One tab of the quest log, gathered over one opening of the window.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Gathered {
    pieces: Vec<Piece>,
}

/// A quest gathered, with the region bar it stands under when that is known.
#[derive(Debug, Clone, PartialEq)]
pub struct GatheredQuest {
    pub quest: LogQuest,
    pub region: Option<String>,
}

fn key(item: &LogItem) -> (bool, String) {
    match item {
        LogItem::Region { name, .. } => (true, squash(name)),
        LogItem::Quest(q) => (false, squash(&q.title)),
    }
}

fn same(a: &LogItem, b: &LogItem) -> bool {
    let ((a_bar, a), (b_bar, b)) = (key(a), key(b));
    a_bar == b_bar && (a == b || strsim::normalized_levenshtein(&a, &b) >= SAME_ROW)
}

impl Piece {
    /// Where `other` lies against this piece, as how far down its first item stands (negative
    /// when it starts higher), when the two overlap in agreeing rows or both hold the top or the
    /// end. The longest overlap wins.
    fn offset_of(&self, other: &Piece) -> Option<isize> {
        let (n, m) = (self.items.len() as isize, other.items.len() as isize);
        if self.top && other.top {
            return Some(0);
        }
        if self.end && other.end {
            return Some(n - m);
        }
        let mut best: Option<(isize, isize)> = None;
        for offset in (1 - m)..n {
            let overlap: Vec<(usize, usize)> = (0..m)
                .filter_map(|j| {
                    let i = offset + j;
                    (0..n).contains(&i).then_some((i as usize, j as usize))
                })
                .collect();
            if overlap.is_empty() || !overlap.iter().all(|&(i, j)| same(&self.items[i], &other.items[j])) {
                continue;
            }
            // A piece at the top can't have rows above it, nor one at the end rows below.
            if (offset < 0 && self.top) || (offset > 0 && other.top) || (offset + m > n && self.end) || (offset + m < n && other.end) {
                continue;
            }
            let length = overlap.len() as isize;
            if best.is_none_or(|(_, top)| length > top) {
                best = Some((offset, length));
            }
        }
        best.map(|(offset, _)| offset)
    }

    /// The two pieces as one, `other` placed at `offset`. Where they overlap, `other` is newer
    /// and wins, so a checkbox ticked since shows ticked.
    fn joined(&self, other: &Piece, offset: isize) -> Piece {
        let (n, m) = (self.items.len() as isize, other.items.len() as isize);
        let start = offset.min(0);
        let stop = n.max(offset + m);
        let items = (start..stop)
            .map(|i| {
                let j = i - offset;
                if (0..m).contains(&j) { other.items[j as usize].clone() } else { self.items[i as usize].clone() }
            })
            .collect();
        Piece {
            items,
            top: (offset <= 0 && other.top) || (offset >= 0 && self.top),
            end: (offset + m >= n && other.end) || (offset + m <= n && self.end),
        }
    }
}

impl Gathered {
    /// Adds what the list shows now, from its top down.
    pub fn add(&mut self, items: &[LogItem], scroll: Scroll) {
        let mut piece = Piece { items: items.to_vec(), top: scroll.at_top, end: scroll.at_end };
        // A view holding the top and the end is the whole list as it stands now.
        if piece.top && piece.end {
            self.pieces = vec![piece];
            return;
        }
        let mut rest = Vec::new();
        for old in std::mem::take(&mut self.pieces) {
            match old.offset_of(&piece) {
                Some(offset) if agrees(&old, &piece, offset) => piece = old.joined(&piece, offset),
                // Both at the top or the end but not alike: the list changed, and the new view
                // stands.
                Some(_) => {}
                None => rest.push(old),
            }
        }
        rest.push(piece);
        self.pieces = rest;
    }

    /// Whether the tab has been seen from top to end, every region holding the quests its bar
    /// counts. A region folded shut shows its bar and none of its quests, so it isn't.
    pub fn whole(&self) -> bool {
        let [piece] = &self.pieces[..] else { return false };
        if !(piece.top && piece.end) {
            return false;
        }
        let mut expected: Option<u32> = None;
        let mut counted = 0u32;
        for item in &piece.items {
            match item {
                LogItem::Region { count, .. } => {
                    if expected.is_some_and(|n| n != counted) {
                        return false;
                    }
                    expected = *count;
                    counted = 0;
                }
                LogItem::Quest(_) => counted += 1,
            }
        }
        expected.is_none_or(|n| n == counted)
    }

    /// Every quest gathered, top to bottom, under its region when one shows above it.
    pub fn quests(&self) -> Vec<GatheredQuest> {
        let mut out = Vec::new();
        for piece in &self.pieces {
            let mut region = None;
            for item in &piece.items {
                match item {
                    LogItem::Region { name, .. } => region = Some(name.clone()),
                    LogItem::Quest(quest) => out.push(GatheredQuest { quest: quest.clone(), region: region.clone() }),
                }
            }
        }
        out
    }

    /// Every region bar gathered, with its count when it read.
    pub fn regions(&self) -> Vec<(String, Option<u32>)> {
        self.pieces
            .iter()
            .flat_map(|p| &p.items)
            .filter_map(|item| match item {
                LogItem::Region { name, count } => Some((name.clone(), *count)),
                LogItem::Quest(_) => None,
            })
            .collect()
    }
}

/// Whether two pieces placed by `offset` agree wherever they overlap.
fn agrees(old: &Piece, new: &Piece, offset: isize) -> bool {
    (0..new.items.len() as isize).all(|j| {
        let i = offset + j;
        !(0..old.items.len() as isize).contains(&i) || same(&old.items[i as usize], &new.items[j as usize])
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::readers::questlog::RowIcon;

    fn region(name: &str, count: u32) -> LogItem {
        LogItem::Region { name: name.into(), count: Some(count) }
    }

    fn quest(title: &str) -> LogItem {
        LogItem::Quest(LogQuest {
            title: title.into(),
            readings: Vec::new(),
            cut: false,
            icon: RowIcon::Ring,
            on_helper: None,
            selected: false,
        })
    }

    const TOP: Scroll = Scroll { at_top: true, at_end: false };
    const MIDDLE: Scroll = Scroll { at_top: false, at_end: false };
    const END: Scroll = Scroll { at_top: false, at_end: true };
    const ALL: Scroll = Scroll { at_top: true, at_end: true };

    /// A made-up Completed tab: two regions, seven quests, nine rows.
    fn tab() -> Vec<LogItem> {
        vec![
            region("Fern Isle", 4),
            quest("Pebble Patrol"),
            quest("Moss on the Stones"),
            quest("The Lantern Keeper"),
            quest("A Letter for Brin"),
            region("Duskwood", 3),
            quest("Owl Feathers"),
            quest("Lost in the Hollow"),
            quest("Bark and Bite"),
        ]
    }

    #[test]
    fn a_list_that_fits_is_whole_at_one_look() {
        let mut gathered = Gathered::default();
        gathered.add(&tab(), ALL);
        assert!(gathered.whole());
        let quests = gathered.quests();
        assert_eq!(quests.len(), 7);
        assert_eq!(quests[4].region.as_deref(), Some("Duskwood"));
    }

    #[test]
    fn scrolled_views_join_into_the_whole_list() {
        let all = tab();
        let mut gathered = Gathered::default();
        gathered.add(&all[0..4], TOP);
        assert!(!gathered.whole());
        gathered.add(&all[3..7], MIDDLE);
        assert!(!gathered.whole());
        gathered.add(&all[5..9], END);
        assert!(gathered.whole());
        assert_eq!(gathered.quests().iter().map(|q| q.quest.title.as_str()).collect::<Vec<_>>()[6], "Bark and Bite");
    }

    #[test]
    fn views_out_of_order_and_a_jump_past_the_middle_still_join_once_the_gap_is_seen() {
        let all = tab();
        let mut gathered = Gathered::default();
        gathered.add(&all[6..9], END);
        gathered.add(&all[0..3], TOP);
        assert!(!gathered.whole(), "the rows between are unseen");
        let lost = |g: &Gathered| g.quests().into_iter().find(|q| q.quest.title == "Lost in the Hollow").unwrap().region;
        // Rows under a region scrolled out of view don't know their region yet.
        assert_eq!(lost(&gathered), None);
        gathered.add(&all[2..7], MIDDLE);
        assert!(gathered.whole());
        assert_eq!(lost(&gathered).as_deref(), Some("Duskwood"));
    }

    #[test]
    fn a_row_read_a_letter_apart_is_the_same_row() {
        let all = tab();
        let mut gathered = Gathered::default();
        gathered.add(&all[0..4], TOP);
        let mut later = all[2..9].to_vec();
        later[1] = quest("The Lantern Keepor");
        gathered.add(&later, END);
        assert!(gathered.whole());
    }

    #[test]
    fn a_region_folded_shut_keeps_the_tab_from_being_whole() {
        let mut gathered = Gathered::default();
        let mut folded = tab();
        folded.drain(1..5);
        gathered.add(&folded, ALL);
        assert!(!gathered.whole(), "Fern Isle counts 4 and shows none");
        let mut unread = tab();
        unread[0] = LogItem::Region { name: "Fern Isle".into(), count: None };
        gathered.add(&unread, ALL);
        assert!(gathered.whole(), "a count that didn't read checks nothing");
    }

    #[test]
    fn an_empty_tab_is_whole() {
        let mut gathered = Gathered::default();
        gathered.add(&[], ALL);
        assert!(gathered.whole());
        assert!(gathered.quests().is_empty());
    }

    #[test]
    fn a_newer_view_of_the_same_rows_wins() {
        let mut gathered = Gathered::default();
        gathered.add(&tab()[0..4], TOP);
        let mut ticked = tab()[0..4].to_vec();
        if let LogItem::Quest(q) = &mut ticked[1] {
            q.on_helper = Some(true);
        }
        gathered.add(&ticked, TOP);
        assert_eq!(gathered.quests()[0].quest.on_helper, Some(true));
    }
}
