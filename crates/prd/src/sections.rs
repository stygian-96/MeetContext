//! Splits a PRD into flat sections (each heading runs to the next heading of any level)
//! and matches a new version's sections to the old ones so section IDs survive edits.

use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq)]
pub struct Section {
    /// '' for text before the first heading.
    pub heading: String,
    /// 1–6 for `#`..`######`; 0 for the intro text.
    pub level: u8,
    /// The whole section, heading line included.
    pub text: String,
}

/// ATX headings only (`## Scope`); `#` lines inside ``` or ~~~ code fences are not headings.
pub fn split(md: &str) -> Vec<Section> {
    let mut out = vec![];
    let mut cur = Section { heading: String::new(), level: 0, text: String::new() };
    let mut fence: Option<&str> = None;
    for line in md.split_inclusive('\n') {
        let t = line.trim_start();
        match fence {
            Some(f) if t.starts_with(f) => fence = None,
            Some(_) => {}
            None if t.starts_with("```") => fence = Some("```"),
            None if t.starts_with("~~~") => fence = Some("~~~"),
            None => {
                if let Some((level, heading)) = atx_heading(line) {
                    if cur.level > 0 || !cur.text.trim().is_empty() {
                        out.push(cur);
                    }
                    cur = Section { heading, level, text: String::new() };
                }
            }
        }
        cur.text.push_str(line);
    }
    if cur.level > 0 || !cur.text.trim().is_empty() {
        out.push(cur);
    }
    out
}

fn atx_heading(line: &str) -> Option<(u8, String)> {
    let line = line.trim_end();
    let indent = line.len() - line.trim_start_matches(' ').len();
    let rest = &line[indent..];
    let level = rest.len() - rest.trim_start_matches('#').len();
    let after = &rest[level..];
    if indent > 3 || !(1..=6).contains(&level) || !(after.is_empty() || after.starts_with([' ', '\t'])) {
        return None;
    }
    Some((level as u8, after.trim().trim_end_matches('#').trim_end().to_string()))
}

/// For each new section, the index of the old section it continues, if any.
/// Order of preference: same heading and level, same heading, then the most similar body.
pub fn match_sections(old: &[Section], new: &[Section]) -> Vec<Option<usize>> {
    let mut used = HashSet::new();
    let mut out = vec![None; new.len()];
    // Strict pass: same heading and level; loose pass: same heading at any level.
    for strict in [true, false] {
        let same = |o: &Section, n: &Section| o.heading == n.heading && if strict { o.level == n.level } else { n.level > 0 };
        for (i, n) in new.iter().enumerate() {
            if out[i].is_none()
                && let Some(j) = (0..old.len()).find(|j| !used.contains(j) && same(&old[*j], n))
            {
                used.insert(j);
                out[i] = Some(j);
            }
        }
    }
    // A renamed heading: keep the ID when the body is mostly the same.
    for (i, n) in new.iter().enumerate() {
        if out[i].is_none() {
            let best = (0..old.len())
                .filter(|j| !used.contains(j))
                .map(|j| (j, similarity(&body(&old[j]), &body(n))))
                .filter(|&(_, s)| s >= 0.5)
                .max_by(|a, b| a.1.total_cmp(&b.1));
            if let Some((j, _)) = best {
                used.insert(j);
                out[i] = Some(j);
            }
        }
    }
    out
}

fn body(s: &Section) -> String {
    if s.level == 0 { s.text.clone() } else { s.text.split_once('\n').map_or("", |x| x.1).to_string() }
}

/// Word-set overlap (Jaccard), 0..1.
fn similarity(a: &str, b: &str) -> f64 {
    let words = |s: &str| s.split_whitespace().map(str::to_lowercase).collect::<HashSet<_>>();
    let (a, b) = (words(a), words(b));
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    a.intersection(&b).count() as f64 / a.union(&b).count() as f64
}

/// Stable across Rust versions (unlike std's hasher), so hashes can be stored.
pub fn hash(text: &str) -> String {
    let h = text.bytes().fold(0xcbf29ce484222325u64, |h, b| (h ^ u64::from(b)).wrapping_mul(0x100000001b3));
    format!("{h:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &str = "Intro line.\n\n## Overview\nCheckout as one page.\n\n## Scope\nCards, guest checkout and saved cards.\n\n### Guest checkout\nEmail on the payment step.\n\n```md\n## not a heading\n```\n";

    fn headings(md: &str) -> Vec<(u8, String)> {
        split(md).into_iter().map(|s| (s.level, s.heading)).collect()
    }

    #[test]
    fn splits_flat_sections_and_skips_code_fences() {
        assert_eq!(
            headings(DOC),
            [(0, ""), (2, "Overview"), (2, "Scope"), (3, "Guest checkout")].map(|(l, h)| (l, h.to_string()))
        );
        assert_eq!(split(DOC).iter().map(|s| s.text.as_str()).collect::<String>(), DOC, "sections cover the whole text");
        assert!(split(DOC)[3].text.contains("## not a heading"));
        // Not headings, so all of it is intro text.
        assert_eq!(headings("#NoSpace\n####### seven\n    ## indented code\n"), [(0, String::new())]);
        assert_eq!(headings("## Closed ##\n"), [(2, "Closed".to_string())]);
    }

    #[test]
    fn ids_survive_edits_reorders_and_renames() {
        let old = split(DOC);
        // Body edited, sections reordered, one heading renamed, one section added, one removed.
        let new = split(
            "Intro line.\n\n## Scope\nCards, guest checkout, saved cards and wallets.\n\n## Summary\nCheckout as one page.\n\n## Risks\nFraud.\n",
        );
        assert_eq!(match_sections(&old, &new), [Some(0), Some(2), Some(1), None]);
    }

    #[test]
    fn duplicate_headings_match_in_order() {
        let old = split("## Notes\na\n## Notes\nb\n");
        let new = split("## Notes\na\n## Notes\nb changed\n");
        assert_eq!(match_sections(&old, &new), [Some(0), Some(1)]);
    }

    #[test]
    fn hash_is_stable() {
        assert_eq!(hash(""), "cbf29ce484222325");
        assert_ne!(hash("a"), hash("b"));
    }
}
