//! A problem that names where it is (#432, research/MODULE-WIRING.md §5): a cable, a node, an address, a track – so an
//! editor marks the thing, not the document. The checks that find several (`Graph::build`, `Rig::check`) list them all
//! as one `Problems` error instead of stopping at the first; `sentences` reads them back out of any `anyhow::Error`, the
//! contexts added on the way (`rig X flattened`) before each, so a single problem reads as it always did (`{e:#}`).
use serde::{Deserialize, Serialize};
use std::fmt;

/// One problem: where (`at`: `link a.out → b.in`, `node 'x'`, `route voice:melody.cv → drone:drone.freq`, `persona
/// calm`; "" = the whole document) and what (`says`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Problem {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub at: String,
    pub says: String,
}

impl Problem {
    pub fn new(at: impl Into<String>, says: impl Into<String>) -> Problem {
        Problem { at: at.into(), says: says.into() }
    }

    /// an error that names its place in its own words (or none): the whole sentence, no `at`
    pub fn of(e: &anyhow::Error) -> Problem {
        Problem { at: String::new(), says: format!("{e:#}") }
    }
}

impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.at.is_empty() {
            f.write_str(&self.says)
        } else {
            write!(f, "{}: {}", self.at, self.says)
        }
    }
}

/// Every problem a check found, as one error (`Display`: joined by `; `).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Problems(pub Vec<Problem>);

impl Problems {
    pub fn push(&mut self, at: impl Into<String>, says: impl Into<String>) {
        self.0.push(Problem::new(at, says));
    }

    /// an error's own sentence as a problem of the whole (`Problem::of`)
    pub fn note(&mut self, e: anyhow::Error) {
        self.0.push(Problem::of(&e));
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// `Ok` with none, else all of them as the error
    pub fn into_result(self) -> anyhow::Result<()> {
        if self.0.is_empty() {
            Ok(())
        } else {
            Err(anyhow::Error::new(self))
        }
    }
}

impl fmt::Display for Problems {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, p) in self.0.iter().enumerate() {
            if i > 0 {
                f.write_str("; ")?;
            }
            write!(f, "{p}")?;
        }
        Ok(())
    }
}

impl std::error::Error for Problems {}

/// The problems in `e`: a `Problems` root gives each of its own, anything else one problem with the whole sentence.
/// The contexts above the root (`e.context(...)`) are not here – `sentences` puts them before each.
pub fn listed(e: &anyhow::Error) -> Vec<Problem> {
    match e.root_cause().downcast_ref::<Problems>() {
        Some(p) => p.0.clone(),
        None => vec![Problem::of(e)],
    }
}

/// Each problem of `e` as a full sentence, the contexts above the root before it: for one problem exactly `{e:#}`.
pub fn sentences(e: &anyhow::Error) -> Vec<String> {
    let Some(p) = e.root_cause().downcast_ref::<Problems>() else { return vec![format!("{e:#}")] };
    let links: Vec<String> = e.chain().map(|x| x.to_string()).collect();
    // the chain is the contexts, outermost first, then the root's own text
    let prefix = links[..links.len().saturating_sub(1)].join(": ");
    p.0.iter().map(|p| if prefix.is_empty() { p.to_string() } else { format!("{prefix}: {p}") }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_problem_reads_as_one_sentence_and_several_as_a_list() {
        let mut ps = Problems::default();
        assert!(ps.clone().into_result().is_ok());
        ps.push("link a.out → b.in", "the input already has a link");
        ps.push("", "the graph has a cycle: a → b → a");
        let e = ps.into_result().unwrap_err().context("rig character.x flattened");
        assert_eq!(format!("{e:#}"), "rig character.x flattened: link a.out → b.in: the input already has a link; the graph has a cycle: a → b → a");
        assert_eq!(
            sentences(&e),
            ["rig character.x flattened: link a.out → b.in: the input already has a link", "rig character.x flattened: the graph has a cycle: a → b → a"]
        );
        assert_eq!(listed(&e)[0], Problem::new("link a.out → b.in", "the input already has a link"));
        // a plain error is one problem of the whole
        let plain = anyhow::anyhow!("no instrument 'x'").context("track 'voice'");
        assert_eq!(sentences(&plain), ["track 'voice': no instrument 'x'"]);
        assert_eq!(listed(&plain), [Problem::new("", "track 'voice': no instrument 'x'")]);
    }
}
