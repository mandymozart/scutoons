//! The vote's one tie rule (LIVE.md B.6, decision B.11 (3)), read by the relay's tally (relay/vote.rs) and by the
//! conductor's panel (deck-show vote.rs, a group's pick) – moved here from the relay so it exists once (#533).

/// How many casts each of `n` options got.
pub fn count(n: usize, options: impl Iterator<Item = usize>) -> Vec<i64> {
    let mut counts = vec![0; n];
    for o in options {
        if let Some(c) = counts.get_mut(o) {
            *c += 1;
        }
    }
    counts
}

/// The option with the most votes; a tie goes to the fallback if it is among the tied, else to the first tied option.
pub fn winner(counts: &[i64], fallback: usize) -> usize {
    let max = counts.iter().copied().max().unwrap_or(0);
    if counts.get(fallback) == Some(&max) {
        return fallback;
    }
    counts.iter().position(|&c| c == max).unwrap_or(fallback)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_most_votes_win_and_a_tie_goes_to_the_fallback_when_it_is_tied() {
        assert_eq!(count(3, [0, 1, 1, 7].into_iter()), [1, 2, 0], "an option out of range is not counted");
        assert_eq!(winner(&[1, 2, 0], 0), 1);
        assert_eq!(winner(&[2, 2, 0], 1), 1, "the fallback is tied");
        assert_eq!(winner(&[2, 2, 0], 2), 0, "the fallback is not tied: the first tied");
        assert_eq!(winner(&[], 0), 0);
    }
}
