use regex::Regex;
use super::FilterRule;

pub struct EmailRule {
    re: Regex,
}

impl EmailRule {
    pub fn new() -> Self {
        Self {
            re: Regex::new(r"\b[a-zA-Z0-9._%+\-]+@[a-zA-Z0-9.\-]+\.[a-zA-Z]{2,}\b").unwrap(),
        }
    }
}

impl FilterRule for EmailRule {
    fn apply(&self, text: &str) -> String {
        self.re.replace_all(text, "[email masqué]").to_string()
    }
}
