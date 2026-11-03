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

/// The `(file, function)` pairs in a README's covers block, or the line that
/// isn't `<file>::<function>`. A README without the block covers nothing.
pub fn covers(readme: &str) -> Vec<Result<(&str, &str), &str>> {
    block(readme, "covers")
        .map(|line| line.split_once("::").ok_or(line))
        .collect()
}

/// The `<kind> <name>` lines in a README's surface block, or the line that
/// isn't in that form. A README without the block has no surface.
pub fn surface(readme: &str) -> Vec<Result<&str, &str>> {
    block(readme, "surface")
        .map(|line| match line.split_once(' ') {
            Some((_, name)) if !name.contains(' ') => Ok(line),
            _ => Err(line),
        })
        .collect()
}

/// The non-blank, trimmed lines of a README's first ```` ```kind ```` block.
fn block<'a>(readme: &'a str, kind: &str) -> impl Iterator<Item = &'a str> {
    readme
        .split_once(&format!("```{kind}\n"))
        .and_then(|(_, rest)| rest.split_once("```"))
        .map_or("", |(block, _)| block)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
}

/// The 1-based numbers of the lines that are `// Contract:` tags.
pub fn tag_lines(src: &str) -> Vec<usize> {
    src.lines()
        .enumerate()
        .filter(|(_, line)| line.trim_start().starts_with(TAG))
        .map(|(n, _)| n + 1)
        .collect()
}

pub struct TaggedTest {
    pub name: String,
    pub tags: Vec<String>,
    pub ignored: bool,
}

fn is_ignore(line: &str) -> bool {
    matches!(
        line.strip_prefix("#[ignore")
            .and_then(|rest| rest.trim_start().chars().next()),
        Some(']' | '=')
    )
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
        let attrs: Vec<&str> = above
            .iter()
            .rev()
            .map(|l| l.trim())
            .take_while(|l| l.starts_with("//") || l.starts_with("#["))
            .collect();
        let tags = attrs
            .iter()
            .filter_map(|l| l.strip_prefix(TAG))
            .flat_map(|t| t.split(',').map(|s| s.trim().to_string()))
            .collect();
        let ignored = attrs
            .into_iter()
            .chain(
                below
                    .iter()
                    .map(|l| l.trim())
                    .take_while(|l| !l.starts_with("fn ")),
            )
            .any(is_ignore);
        tests.push(TaggedTest {
            name,
            tags,
            ignored,
        });
    }
    tests
}
