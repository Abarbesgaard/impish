//! Tries mutants on several scratch copies at once, and hands back their
//! outcomes in the order a serial run would.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::thread;

use super::scratch::Scratch;

/// Runs `work` on items `0..n`, one at a time on each of the `scratches`,
/// and calls `done` with each result in item order.
pub(super) fn each<T: Send>(
    scratches: &[Scratch],
    n: usize,
    work: impl Fn(&Scratch, usize) -> anyhow::Result<T> + Sync,
    mut done: impl FnMut(T),
) -> anyhow::Result<()> {
    let next = AtomicUsize::new(0);
    thread::scope(|s| {
        let (tx, rx) = mpsc::channel();
        for scratch in scratches {
            let (tx, next, work) = (tx.clone(), &next, &work);
            s.spawn(move || {
                loop {
                    let i = next.fetch_add(1, Ordering::SeqCst);
                    if i >= n {
                        break;
                    }
                    // Once the run has stopped on an error, nobody listens.
                    if tx.send((i, work(scratch, i))).is_err() {
                        break;
                    }
                }
            });
        }
        drop(tx);
        let mut waiting = BTreeMap::new();
        let mut reported = 0;
        for (i, result) in rx {
            waiting.insert(i, result?);
            while let Some(t) = waiting.remove(&reported) {
                done(t);
                reported += 1;
            }
        }
        Ok(())
    })
}
