//! Which contract tests run each mutant's code: a probe at the start of every
//! block of the covered functions, then each contract test on its own.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::thread;

use anyhow::Context;
use proc_macro2::LineColumn;
use syn::visit::{self, Visit};

use super::Mutant;
use super::scratch::{Scratch, offset};
use crate::code::Function;

/// A block's file, and its first and last character as line and 0-based
/// column.
type Block = (String, (usize, usize), (usize, usize));

pub(super) struct Reach {
    blocks: Vec<Block>,
    /// Per contract test, the blocks it entered.
    hits: BTreeMap<String, BTreeSet<usize>>,
}

impl Reach {
    /// The contract tests that run the mutant's code, or `None` when no
    /// probed block holds it.
    pub(super) fn ran(&self, m: &Mutant) -> Option<BTreeSet<String>> {
        let start = (m.span.start.line, m.span.start.column - 1);
        let end = (m.span.end.line, m.span.end.column - 1);
        let (block, _) = self
            .blocks
            .iter()
            .enumerate()
            .filter(|(_, (file, a, b))| *file == m.file && *a <= start && end <= *b)
            .max_by_key(|(_, (_, a, _))| *a)?;
        Some(
            self.hits
                .iter()
                .filter(|(_, hit)| hit.contains(&block))
                .map(|(test, _)| test.clone())
                .collect(),
        )
    }
}

/// Probes the blocks of the `owned` functions in the scratch copy, builds
/// it, runs each of the `tests` alone up to `jobs` at once, and puts the
/// code back. `None` when the probed copy doesn't build.
pub(super) fn reach(
    scratch: &Scratch,
    owned: &[Function],
    tests: &[&str],
    jobs: usize,
) -> anyhow::Result<Option<Reach>> {
    let files: BTreeSet<&str> = owned.iter().map(|f| f.file.as_str()).collect();
    let mut blocks = Vec::new();
    let mut originals = Vec::new();
    for file in files {
        let path = scratch.dir.join(file);
        let src = fs::read_to_string(&path).with_context(|| format!("read {file}"))?;
        let Ok(ast) = syn::parse_file(&src) else {
            continue;
        };
        let lines: Vec<_> = owned
            .iter()
            .filter(|f| f.file == file)
            .map(|f| (f.first, f.last))
            .collect();
        let mut found = Blocks { lines, at: vec![] };
        found.visit_file(&ast);
        let mut probed = src.clone();
        // From the end, so each insertion leaves the offsets before it.
        for (start, end) in found.at.into_iter().rev() {
            let Some(at) = offset(&src, start.line, start.column + 1) else {
                continue;
            };
            probed.insert_str(at + 1, &probe(blocks.len()));
            let to = |p: LineColumn| (p.line, p.column);
            blocks.push((file.to_string(), to(start), to(end)));
        }
        fs::write(&path, probed)?;
        originals.push((path, src));
    }
    let built = scratch.build();
    let hits = match &built {
        Ok(Some(built)) => Some(run(scratch, &built.contract, tests, jobs)?),
        _ => None,
    };
    for (path, src) in originals {
        fs::write(path, src)?;
    }
    built?;
    Ok(hits.map(|hits| Reach { blocks, hits }))
}

/// Runs each test alone, its probes writing to a file of its own.
fn run(
    scratch: &Scratch,
    exe: &std::path::Path,
    tests: &[&str],
    jobs: usize,
) -> anyhow::Result<BTreeMap<String, BTreeSet<usize>>> {
    let next = Mutex::new(tests.iter().enumerate());
    let hits = Mutex::new(BTreeMap::new());
    thread::scope(|s| {
        let workers: Vec<_> = (0..jobs.max(1))
            .map(|_| {
                s.spawn(|| -> anyhow::Result<()> {
                    loop {
                        let item = next.lock().expect("lock").next();
                        let Some((i, test)) = item else { break };
                        let file = scratch.dir.with_extension(format!("reach-{i}"));
                        let _ = fs::remove_file(&file);
                        Command::new(exe)
                            .args([test, "--exact"])
                            .current_dir(&scratch.dir)
                            .env("IMPISH_REACH", &file)
                            .stdout(Stdio::null())
                            .stderr(Stdio::null())
                            .status()
                            .context("run a contract test alone")?;
                        let hit = fs::read_to_string(&file).unwrap_or_default();
                        let _ = fs::remove_file(&file);
                        let hit = hit.lines().filter_map(|l| l.parse().ok()).collect();
                        hits.lock().expect("lock").insert(test.to_string(), hit);
                    }
                    Ok(())
                })
            })
            .collect();
        workers
            .into_iter()
            .try_for_each(|w| w.join().expect("a reach worker panicked"))
    })?;
    Ok(hits.into_inner().expect("lock"))
}

/// A statement that writes its block's number to `IMPISH_REACH` the first
/// time the block is entered.
fn probe(id: usize) -> String {
    format!(
        "{{ static R: ::std::sync::atomic::AtomicBool = ::std::sync::atomic::AtomicBool::new(false); \
         if !R.swap(true, ::std::sync::atomic::Ordering::Relaxed) {{ \
         if let Some(p) = ::std::env::var_os(\"IMPISH_REACH\") {{ \
         if let Ok(mut f) = ::std::fs::OpenOptions::new().create(true).append(true).open(p) {{ \
         let _ = ::std::io::Write::write_all(&mut f, b\"{id}\\n\"); }} }} }} }}"
    )
}

/// The blocks inside the lines of owned functions, leaving out const
/// contexts, where a probe can't run.
struct Blocks {
    lines: Vec<(usize, usize)>,
    at: Vec<(LineColumn, LineColumn)>,
}

impl<'ast> Visit<'ast> for Blocks {
    fn visit_block(&mut self, b: &'ast syn::Block) {
        let (start, end) = (
            b.brace_token.span.open().start(),
            b.brace_token.span.close().end(),
        );
        if self
            .lines
            .iter()
            .any(|&(first, last)| first <= start.line && start.line <= last)
        {
            self.at.push((start, end));
        }
        visit::visit_block(self, b);
    }
    fn visit_signature(&mut self, _: &'ast syn::Signature) {}
    fn visit_item_fn(&mut self, f: &'ast syn::ItemFn) {
        if f.sig.constness.is_none() {
            visit::visit_item_fn(self, f);
        }
    }
    fn visit_impl_item_fn(&mut self, f: &'ast syn::ImplItemFn) {
        if f.sig.constness.is_none() {
            visit::visit_impl_item_fn(self, f);
        }
    }
    fn visit_item_const(&mut self, _: &'ast syn::ItemConst) {}
    fn visit_item_static(&mut self, _: &'ast syn::ItemStatic) {}
    fn visit_expr_const(&mut self, _: &'ast syn::ExprConst) {}
}
