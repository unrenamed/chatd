pub struct KMP {
    pattern: String,
    lps: Vec<usize>,
}

impl KMP {
    pub fn new(pattern: &str) -> Self {
        let lps = KMP::compute_lps(pattern);
        KMP {
            pattern: pattern.to_string(),
            lps,
        }
    }

    pub fn search(&self, text: &str) -> Vec<usize> {
        let mut result = Vec::new();
        let mut i = 0;
        let mut j = 0;

        while i < text.len() {
            if self.pattern.chars().nth(j).unwrap() == text.chars().nth(i).unwrap() {
                i += 1;
                j += 1;
            }

            if j == self.pattern.len() {
                result.push(i - j);
                j = self.lps[j - 1];
            } else if i < text.len()
                && self.pattern.chars().nth(j).unwrap() != text.chars().nth(i).unwrap()
            {
                if j != 0 {
                    j = self.lps[j - 1];
                } else {
                    i += 1;
                }
            }
        }

        result
    }

    fn compute_lps(pattern: &str) -> Vec<usize> {
        let mut lps = vec![0; pattern.len()];
        let mut len = 0;
        let mut i = 1;

        while i < pattern.len() {
            if pattern.chars().nth(i).unwrap() == pattern.chars().nth(len).unwrap() {
                len += 1;
                lps[i] = len;
                i += 1;
            } else {
                if len != 0 {
                    len = lps[len - 1];
                } else {
                    lps[i] = 0;
                    i += 1;
                }
            }
        }

        lps
    }
}
