//! `cargo impish regen <primitive>`: a pack to rebuild a primitive from its
//! Contract alone. It holds the intents, the obligations, the decision
//! records, the contract tests and stubs of the covered functions, and none
//! of their code.

mod stubs;

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use std::process::ExitCode;

use anyhow::{Context, bail};

use crate::code;
use crate::contract::escapes;

/// Lists what a pack's run wrote, so the next run deletes only that.
const MARKER: &str = ".impish-pack";

/// How a Contract is written, as impish reads it.
const FORMAT: &str = include_str!("../contract/README.md");

pub fn run(root: &Path, primitive: &str, out: Option<&Path>) -> anyhow::Result<ExitCode> {
    let readme_file = format!("contract/{primitive}/README.md");
    let named = !primitive.is_empty() && !primitive.contains(['/', '\\']) && !escapes(primitive);
    let Some(_) = named
        .then(|| fs::read_to_string(root.join(&readme_file)).ok())
        .flatten()
    else {
        bail!("no {readme_file}: `{primitive}` isn't a primitive");
    };
    let out = out.map_or_else(
        || root.join("target/impish/regen").join(primitive),
        Path::to_path_buf,
    );
    let pack = pack(root, primitive);
    clear(&out)?;
    for (rel, text) in &pack {
        let path = out.join(rel);
        fs::create_dir_all(path.parent().unwrap_or(&out))?;
        fs::write(&path, text).with_context(|| format!("write {}", path.display()))?;
    }
    let listed: Vec<&str> = pack.keys().map(String::as_str).collect();
    fs::write(out.join(MARKER), listed.join("\n") + "\n")?;
    println!(
        "Wrote the {primitive} pack to {}: {} files. Hand it to an agent that hasn't seen the code.",
        out.display(),
        pack.len()
    );
    Ok(ExitCode::SUCCESS)
}

/// The pack's files, by path inside it.
fn pack(root: &Path, primitive: &str) -> BTreeMap<String, String> {
    let mut pack = BTreeMap::from([
        ("PROMPT.md".to_string(), prompt(primitive)),
        ("FORMAT.md".to_string(), FORMAT.to_string()),
    ]);
    let copied = [
        "contract/intent.md".to_string(),
        "contract/decisions.toml".to_string(),
        format!("contract/{primitive}/README.md"),
        format!("contract/{primitive}/intent.md"),
        format!("contract/{primitive}/decisions.toml"),
        format!("tests/contract/{primitive}.rs"),
    ];
    for rel in copied {
        if let Ok(text) = fs::read_to_string(root.join(&rel)) {
            pack.insert(rel, text);
        }
    }
    let code = code::resolve(root, &code::readmes(root));
    let mut covered: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
    for f in code.owned.get(primitive).into_iter().flatten() {
        covered.entry(&f.file).or_default().push(f.first);
    }
    for (file, firsts) in covered {
        let stubs = fs::read_to_string(root.join(file))
            .ok()
            .and_then(|src| stubs::stubs(&src, &firsts));
        if let Some(stubs) = stubs {
            pack.insert(format!("stubs/{file}"), stubs);
        }
    }
    pack
}

fn prompt(primitive: &str) -> String {
    format!(
        "# Rebuild `{primitive}`\n\n\
         Rebuild the `{primitive}` primitive from this pack alone, without its old code.\n\n\
         - `FORMAT.md`: how a Contract is written.\n\
         - `contract/intent.md` and `contract/{primitive}/intent.md`, when present: why it \
         exists and what it exposes.\n\
         - `contract/{primitive}/README.md`: its obligations. Every one must hold.\n\
         - `contract/decisions.toml` and `contract/{primitive}/decisions.toml`, when present: \
         why it is built the way it is.\n\
         - `tests/contract/{primitive}.rs`: the contract tests. Make them pass without \
         changing them.\n\
         - `stubs/`: the signatures of the functions the Contract covers, each with a \
         `todo!()` body. Keep the signatures and write the bodies.\n\n\
         Whenever you have to guess something the pack doesn't say, write the guess down: \
         each one is a sentence the Contract is missing.\n"
    )
}

/// Empties `out` of a previous pack, deleting only what that pack's marker
/// lists, and refuses, deleting nothing, a directory impish didn't fill.
fn clear(out: &Path) -> anyhow::Result<()> {
    let mut present = Vec::new();
    files(out, "", &mut present);
    if present.is_empty() {
        return Ok(());
    }
    let Ok(marker) = fs::read_to_string(out.join(MARKER)) else {
        bail!(
            "{} isn't empty and has no {MARKER}, so it isn't a pack; deleted nothing",
            out.display()
        );
    };
    let written: BTreeSet<&str> = marker.lines().chain([MARKER]).collect();
    if let Some(stray) = present.iter().find(|f| !written.contains(f.as_str())) {
        bail!(
            "{} holds {stray}, which impish didn't write; deleted nothing",
            out.display()
        );
    }
    for file in &present {
        fs::remove_file(out.join(file))?;
    }
    Ok(())
}

/// Every file under `dir`, as a path relative to the pack with `/` between
/// its parts. A symlink is a file here: it is never followed.
fn files(dir: &Path, prefix: &str, out: &mut Vec<String>) {
    for entry in fs::read_dir(dir).into_iter().flatten().flatten() {
        let rel = format!("{prefix}{}", entry.file_name().to_string_lossy());
        if entry.file_type().is_ok_and(|t| t.is_dir()) {
            files(&entry.path(), &format!("{rel}/"), out);
        } else {
            out.push(rel);
        }
    }
}
