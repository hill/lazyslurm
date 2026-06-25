use std::time::{SystemTime, UNIX_EPOCH};

pub type Quote = (&'static str, &'static str);

pub const QUOTES: &[Quote] = &[
    (
        "We do not remember days; we remember moments.",
        "Cesare Pavese",
    ),
    (
        "Nothing is particularly hard if you divide it into small jobs.",
        "Henry Ford",
    ),
    ("Patience is bitter, but its fruit is sweet.", "Aristotle"),
    (
        "It is not enough to be busy; so are the ants.",
        "Henry David Thoreau",
    ),
    (
        "Time you enjoy wasting is not wasted time.",
        "Marthe Troly-Curtin",
    ),
    ("Slow is smooth, and smooth is fast.", "Anon"),
    ("Simplicity is the soul of efficiency.", "Austin Freeman"),
    (
        "Premature optimization is the root of all evil.",
        "Donald Knuth",
    ),
    ("Waiting is one of the great arts.", "Margery Allingham"),
    (
        "The best way to predict the future is to invent it.",
        "Alan Kay",
    ),
];

/// Pick a quote using the wall clock as a cheap seed. No need for a real RNG
/// dependency just to rotate a handful of strings.
pub fn pick() -> Quote {
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    QUOTES[(seed as usize) % QUOTES.len()]
}
