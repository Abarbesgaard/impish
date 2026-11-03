//! Reading the Contract: obligations in a README, tags in a test file.

use std::collections::BTreeSet;

const TAG: &str = "// Contract:";

/// The obligation numbers a primitive's README declares. `file` names the
/// README in errors.
pub fn obligations(readme: &str, file: &str) -> Result<BTreeSet<u32>, String> {
    let mut ids = BTreeSet::new();
    for (n, line) in readme.lines().enumerate() {
        let Some(rest) = line.strip_prefix("- **A") else {
            continue;
        };
        let num = rest
            .split_once("** ")
            .filter(|(_, text)| !text.trim().is_empty())
            .and_then(|(num, _)| num.parse::<u32>().ok())
            .ok_or_else(|| format!("{file}:{}: malformed obligation `{line}`", n + 1))?;
        if !ids.insert(num) {
            return Err(format!("{file}:{}: A{num} is declared twice", n + 1));
        }
    }
    if ids.is_empty() {
        return Err(format!("{file}: declares no obligations"));
    }
    Ok(ids)
}

pub struct TaggedTest {
    pub name: String,
    pub tags: Vec<String>,
}

/// Every `#[test]` in a contract test file, with the `// Contract:` tags in
/// the comments and attributes right above it.
pub fn tagged_tests(src: &str) -> Vec<TaggedTest> {
    let lines: Vec<&str> = src.lines().collect();
    let mut tests = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        if line.trim() != "#[test]" {
            continue;
        }
        let (above, below) = lines.split_at(i);
        let name = below
            .iter()
            .find_map(|l| l.trim().strip_prefix("fn "))
            .and_then(|l| l.split('(').next())
            .unwrap_or("?")
            .to_string();
        let tags = above
            .iter()
            .rev()
            .map(|l| l.trim())
            .take_while(|l| l.starts_with("//") || l.starts_with("#["))
            .filter_map(|l| l.strip_prefix(TAG))
            .flat_map(|t| t.split(',').map(|s| s.trim().to_string()))
            .collect();
        tests.push(TaggedTest { name, tags });
    }
    tests
}
