/// the wordlist the attacks are taken from
pub static ATTACK_WORDLIST: &[&str] = &[
    "about", "after", "all", "also", "and", "any", "back", "because", "but", "can", "come",
    "could", "day", "even", "first", "for", "from", "get", "give", "good", "have", "him", "how",
    "into", "its", "just", "know", "like", "look", "make", "most", "new", "not", "now", "one",
    "only", "other", "out", "over", "people", "say", "see", "she", "some", "take", "than", "that",
    "their", "them", "then", "there", "these", "they", "think", "this", "time", "two", "use",
    "want", "way", "well", "what", "when", "which", "who", "will", "with", "work", "would", "year",
    "your",
];

#[derive(Debug, PartialEq)]
pub enum DetectorResult {
    SignatureMatch,
    Clear,
}

pub fn run_detector(flow: &str) -> DetectorResult {
    // run preprocessing
    let tmp = flow.to_lowercase();
    // FIXME: Originally, the authors also envisioned normalization from escape
    // sequences, however this was not used in the benchmark code from STAMP

    // run the signature detection
    for signature in ATTACK_WORDLIST {
        if tmp.contains(signature) {
            return DetectorResult::SignatureMatch;
        }
    }

    DetectorResult::Clear
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_recognition() {
        assert_eq!(run_detector("test"), DetectorResult::Clear);
        assert_eq!(run_detector("abouts"), DetectorResult::SignatureMatch);
    }

    #[test]
    fn capitalization_normalization() {
        assert_eq!(run_detector("aBoUt"), DetectorResult::SignatureMatch);
    }

//     #[test]
//     fn escape_sequence_normalization() {
//         assert_eq!(run_detector("%41Bout"), DetectorResult::SignatureMatch);
//     }
}
