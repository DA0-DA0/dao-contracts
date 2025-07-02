use std::collections::HashMap;

/// Simple regex cache to avoid recompiling patterns
#[derive(Default)]
pub struct RegexCache {
    cache: HashMap<String, regex::Regex>,
}

impl RegexCache {
    pub fn get_or_compile(&mut self, pattern: &str) -> Result<&regex::Regex, regex::Error> {
        if !self.cache.contains_key(pattern) {
            if self.cache.len() > 50 {
                self.cache.clear();
            }
            let regex = regex::Regex::new(pattern)?;
            self.cache.insert(pattern.to_string(), regex);
        }
        Ok(self.cache.get(pattern).unwrap())
    }
}
