//! What `check` adds in GitHub Actions: a job summary in Markdown, and an
//! annotation on each line a problem names.

use std::env;
use std::fmt::Write;
use std::fs;
use std::path::Path;

use super::{Primitive, Report};

pub struct Github {
    /// Where the project sits in the repository: empty, or ending in `/`.
    prefix: String,
    /// The repository at the run's commit, ending in `/`, when known.
    blob: Option<String>,
}

impl Github {
    pub fn from_env(root: &Path) -> Self {
        let prefix = env::var_os("GITHUB_WORKSPACE")
            .and_then(|workspace| {
                let workspace = fs::canonicalize(workspace).ok()?;
                let root = fs::canonicalize(root).ok()?;
                let rel = root.strip_prefix(workspace).ok()?;
                let parts: Vec<String> = rel
                    .components()
                    .map(|c| c.as_os_str().to_string_lossy().into_owned())
                    .collect();
                Some(parts.iter().map(|p| format!("{p}/")).collect())
            })
            .unwrap_or_default();
        let var = |name| env::var(name).ok();
        let blob = match (
            var("GITHUB_SERVER_URL"),
            var("GITHUB_REPOSITORY"),
            var("GITHUB_SHA"),
        ) {
            (Some(server), Some(repo), Some(sha)) => Some(format!("{server}/{repo}/blob/{sha}/")),
            _ => None,
        };
        Github { prefix, blob }
    }

    /// An `::error` workflow command for each problem that names a file.
    pub fn annotations(&self, report: &Report) -> Vec<String> {
        report
            .problems
            .iter()
            .filter_map(|p| Some((p.at.as_ref()?, &p.text)))
            .map(|((file, line), text)| {
                let file = property(&format!("{}{file}", self.prefix));
                format!("::error file={file},line={line}::{}", data(text))
            })
            .collect()
    }

    pub fn summary(&self, report: &Report) -> String {
        let mut s = String::new();
        let (primitives, obligations) = (report.primitives.len(), report.obligations());
        let counts = format!("{primitives} primitives, {obligations} obligations");
        if report.problems.is_empty() {
            s += "## ✅ The Contract holds\n\n> [!TIP]\n";
            let _ = writeln!(s, "> **{counts}, all tested.**");
        } else {
            s += "## ❌ The Contract is not met\n\n> [!CAUTION]\n";
            let _ = writeln!(s, "> **{counts}, {} problems.**", report.problems.len());
        }
        s += "\n| Primitive | Active | Tested | Retired | |\n| :-- | --: | --: | --: | :-: |\n";
        for p in &report.primitives {
            let link = format!(
                "{}{}contract/{}/README.md",
                self.blob.as_deref().unwrap_or(""),
                self.prefix,
                p.name
            );
            let retired = p.obligations.len() - p.active();
            let holds = p.tested() == p.active() && !has_problems(report, Some(&p.name));
            let mark = if holds { "✅" } else { "❌" };
            let (active, tested) = (p.active(), p.tested());
            let _ = writeln!(
                s,
                "| [{}]({link}) | {active} | {tested} | {retired} | {mark} |",
                p.name
            );
        }
        self.problems(report, &mut s);
        s += "\n### Obligations\n";
        for p in &report.primitives {
            obligations_of(p, &mut s);
        }
        s
    }

    fn problems(&self, report: &Report, s: &mut String) {
        if report.problems.is_empty() {
            return;
        }
        *s += "\n### Problems\n";
        let names = report.primitives.iter().map(|p| Some(p.name.as_str()));
        for group in names.chain([None]) {
            if !has_problems(report, group) {
                continue;
            }
            let _ = write!(s, "\n**{}**\n\n", group.unwrap_or("Elsewhere"));
            let mine = report
                .problems
                .iter()
                .filter(|p| p.primitive.as_deref() == group);
            for p in mine {
                let _ = writeln!(s, "- {}  ", markdown(&p.text));
                match &p.at {
                    Some((file, line)) => {
                        let _ = writeln!(s, "  `{}{file}:{line}` · {}", self.prefix, p.fix);
                    }
                    None => {
                        let _ = writeln!(s, "  {}", p.fix);
                    }
                }
            }
        }
    }
}

fn has_problems(report: &Report, primitive: Option<&str>) -> bool {
    report
        .problems
        .iter()
        .any(|p| p.primitive.as_deref() == primitive)
}

fn obligations_of(p: &Primitive, s: &mut String) {
    let _ = write!(
        s,
        "\n<details>\n<summary><b>{}</b> · {} active, {} tested</summary>\n\n\
         | | Obligation | Proved by |\n| :-- | :-- | :-- |\n",
        p.name,
        p.active(),
        p.tested()
    );
    for o in &p.obligations {
        let (id, title) = (o.id, markdown(&o.title));
        let _ = match (o.retired, o.tests.is_empty()) {
            (true, _) => writeln!(s, "| ~~A{id}~~ | ~~{title}~~ | retired |"),
            (false, true) => writeln!(s, "| A{id} | {title} | no test |"),
            (false, false) => {
                let tests: Vec<String> = o.tests.iter().map(|t| format!("`{t}`")).collect();
                writeln!(s, "| A{id} | {title} | {} |", tests.join(", "))
            }
        };
    }
    *s += "\n</details>\n";
}

/// Text safe in a Markdown table cell: no `|` ends the cell, no `<` opens a tag.
fn markdown(text: &str) -> String {
    text.replace('|', "\\|").replace('<', "\\<")
}

/// A workflow command's message, escaped as GitHub's runner expects.
fn data(text: &str) -> String {
    text.replace('%', "%25")
        .replace('\r', "%0D")
        .replace('\n', "%0A")
}

/// A workflow command's property value: a message that also can't hold `:` or `,`.
fn property(text: &str) -> String {
    data(text).replace(':', "%3A").replace(',', "%2C")
}
