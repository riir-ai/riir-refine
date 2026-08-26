// A representative Rust source for KAT meter fixtures (G9a). Word tokens are
// whitespace-delimited runs of non-ASCII-whitespace bytes — comments,
// strings, and identifiers all count as processed code.
use std::collections::HashMap;

const MAX_PASSES: usize = 4;

pub fn apply_edits(source: &str, edits: &[Edit]) -> String {
    let mut out = String::with_capacity(source.len());
    let mut cursor = 0usize;
    for edit in edits {
        out.push_str(&source[cursor..edit.start]);
        out.push_str(&edit.replacement);
        cursor = edit.end;
    }
    out.push_str(&source[cursor..]);
    out
}

pub struct Edit {
    pub start: usize,
    pub end: usize,
    pub replacement: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let edits = vec![Edit {
            start: 4,
            end: 7,
            replacement: "xyz".to_string(),
        }];
        let out = apply_edits("abcd1234efgh", &edits);
        assert_eq!(out, "abcdxyzefgh");
        let counts: HashMap<&str, u32> = HashMap::new();
        assert!(counts.is_empty());
    }
}
