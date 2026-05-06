pub fn levenshtein_distance(a: &str, b: &str) -> usize {
    let b_len = b.chars().count();
    if a.is_empty() {
        return b_len;
    }
    if b.is_empty() {
        return a.chars().count();
    }

    let mut row: Vec<usize> = (0..=b_len).collect();

    for (i, char_a) in a.chars().enumerate() {
        let mut previous_diagonal = row[0];
        row[0] = i + 1;

        for (j, char_b) in b.chars().enumerate() {
            let old_row_j = row[j + 1];

            let cost = if char_a == char_b { 0 } else { 1 };

            row[j + 1] =
                std::cmp::min(std::cmp::min(row[j] + 1, row[j + 1] + 1), previous_diagonal + cost);

            previous_diagonal = old_row_j;
        }
    }

    row[b_len]
}

pub fn find_best_match<'a>(
    input: &str,
    candidates: impl Iterator<Item = &'a String>,
) -> Option<String> {
    let mut best_match: Option<(String, usize)> = None;
    let max_threshold = 3;

    for candidate in candidates {
        let dist = levenshtein_distance(input, candidate);

        if dist <= max_threshold {
            match best_match {
                None => best_match = Some((candidate.clone(), dist)),
                Some((_, best_dist)) if dist < best_dist => {
                    best_match = Some((candidate.clone(), dist));
                }
                _ => {}
            }
        }
    }

    best_match.map(|(name, _)| name)
}

pub fn ordinal(n: usize) -> String {
    let s = n.to_string();
    if s.ends_with('1') && !s.ends_with("11") {
        format!("{}st", n)
    } else if s.ends_with('2') && !s.ends_with("12") {
        format!("{}nd", n)
    } else if s.ends_with('3') && !s.ends_with("13") {
        format!("{}rd", n)
    } else {
        format!("{}th", n)
    }
}
