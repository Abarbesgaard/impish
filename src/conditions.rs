//! `cargo impish conditions`: re-check every decision record's conditions.

use std::path::Path;
use std::process::ExitCode;

use anyhow::bail;

use crate::decisions::{self, Verdict};

pub fn run(root: &Path) -> anyhow::Result<ExitCode> {
    let (records, problems) = decisions::read(root);
    if !problems.is_empty() {
        bail!(
            "the decision records are malformed:\n{}",
            problems.join("\n")
        );
    }
    let (mut ok, mut suspect, mut unchecked) = (0, 0, 0);
    for record in &records {
        let verdicts: Vec<_> = record
            .conditions
            .iter()
            .map(|c| (c, decisions::evaluate(root, c)))
            .collect();
        let bad = verdicts
            .iter()
            .any(|(_, v)| matches!(v, Verdict::Suspect(_)));
        println!("{} {}", record.id, if bad { "suspect" } else { "ok" });
        for (c, verdict) in &verdicts {
            match verdict {
                Verdict::Ok => println!("  ok: {}", c.text),
                Verdict::Suspect(why) => println!("  suspect: {} ({why})", c.text),
                Verdict::Unchecked => {
                    unchecked += 1;
                    println!("  not machine-checkable: {}", c.text);
                }
            }
        }
        if bad {
            suspect += 1;
        } else {
            ok += 1;
        }
    }
    println!(
        "{} decision records: {ok} ok, {suspect} suspect; \
         {unchecked} conditions not machine-checkable",
        records.len()
    );
    Ok(if suspect > 0 {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    })
}
