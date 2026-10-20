//! The company written to its file behind the page (`company::changed`): a change is the
//! session's at once, its file written on a thread of its own a moment later - a drag in the
//! Gantt, an edit after an edit, writes the company once, and no command waits for the file
//! meanwhile. What reads the file (a company read again, a simulation past midnight that saves it
//! itself, the app ending) waits for the write first (`settle`); what could not be written is said
//! on the page (`failed`).

use hub_core::company::{store, Company};
use std::path::PathBuf;
use std::sync::{Condvar, Mutex, MutexGuard, OnceLock};
use std::time::{Duration, Instant};

/// How long a change waits for the next before its company is written.
const QUIET: Duration = Duration::from_millis(400);
/// How long changes coming one after another wait at most.
const LONGEST: Duration = Duration::from_secs(3);

#[derive(Default)]
struct Queue {
    /// The companies to write (the latest of each, with the data folder it goes into).
    waiting: Vec<(PathBuf, Company)>,
    /// When the first of them came, and the last.
    first: Option<Instant>,
    last: Option<Instant>,
    /// A write is under way (its companies taken out of `waiting`).
    writing: bool,
    /// What could not be written, to be said.
    failed: Vec<String>,
    /// The writing thread is there.
    started: bool,
}

struct Saver {
    q: Mutex<Queue>,
    cv: Condvar,
}

fn saver() -> &'static Saver {
    static S: OnceLock<Saver> = OnceLock::new();
    S.get_or_init(|| Saver { q: Mutex::new(Queue::default()), cv: Condvar::new() })
}

fn lock() -> MutexGuard<'static, Queue> {
    saver().q.lock().unwrap_or_else(|e| e.into_inner())
}

/// Company `c` to be written into data folder `data` (a company waiting is written as it is now).
pub fn queue(data: PathBuf, c: Company) {
    // (the website: one thread, the file written at once)
    if !cfg!(feature = "native") {
        if let Err(e) = store::save(&data, &c) {
            lock().failed.push(format!("{e:#}"));
        }
        return;
    }
    let mut q = lock();
    match q.waiting.iter_mut().find(|w| w.0 == data && w.1.id == c.id) {
        Some(w) => w.1 = c,
        None => q.waiting.push((data, c)),
    }
    let now = Instant::now();
    q.first.get_or_insert(now);
    q.last = Some(now);
    if !q.started {
        q.started = true;
        std::thread::Builder::new().name("company-saver".into()).spawn(run).expect("the company's writer");
    }
    saver().cv.notify_all();
}

/// The writing thread: what waits written once nothing more came for a moment.
fn run() {
    let s = saver();
    let mut q = lock();
    loop {
        let (Some(first), Some(last)) = (q.first, q.last) else {
            q = s.cv.wait(q).unwrap_or_else(|e| e.into_inner());
            continue;
        };
        let due = (last + QUIET).min(first + LONGEST);
        let now = Instant::now();
        if now < due {
            q = s.cv.wait_timeout(q, due - now).unwrap_or_else(|e| e.into_inner()).0;
            continue;
        }
        q = write_waiting(q);
    }
}

/// What waits written (the queue let go meanwhile), the queue back.
fn write_waiting(mut q: MutexGuard<'static, Queue>) -> MutexGuard<'static, Queue> {
    let batch = std::mem::take(&mut q.waiting);
    (q.first, q.last) = (None, None);
    q.writing = true;
    drop(q);
    let failed: Vec<String> = batch.iter().filter_map(|(data, c)| store::save(data, c).err()).map(|e| format!("{e:#}")).collect();
    for e in &failed {
        hub_core::log_line(&format!("company: {e}"));
    }
    let mut q = lock();
    q.writing = false;
    q.failed.extend(failed);
    saver().cv.notify_all();
    q
}

/// Everything waiting written now, and a write under way waited for: the files are as the
/// session has them (before a file is read, a simulation saves it itself, the app ends).
pub fn settle() {
    let s = saver();
    let mut q = lock();
    loop {
        while q.writing {
            q = s.cv.wait(q).unwrap_or_else(|e| e.into_inner());
        }
        if q.waiting.is_empty() {
            return;
        }
        q = write_waiting(q);
    }
}

/// Company `id` deleted: its write waiting dropped (a write under way waited for), lest its
/// file come back.
pub fn forget(id: &str) {
    let s = saver();
    let mut q = lock();
    q.waiting.retain(|w| w.1.id != id);
    while q.writing {
        q = s.cv.wait(q).unwrap_or_else(|e| e.into_inner());
    }
}

/// What could not be written since asked last.
pub fn failed() -> Vec<String> {
    std::mem::take(&mut lock().failed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn changes_in_a_row_are_written_once_and_settle_writes_at_once() {
        let data = std::env::temp_dir().join(format!("omsihub-saver-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&data);
        let mut c = hub_core::company::found(&hub_core::company::Founding { name: "A".into(), date: "2024-05-06".into(), ..Default::default() }, "Luc");
        c.id = "saver-test".into();
        queue(data.clone(), c.clone());
        c.name = "B".into();
        queue(data.clone(), c.clone());
        // (not yet: it waits for the next change a moment)
        assert!(store::load(&data, &c.id).is_err());
        settle();
        assert_eq!(store::load(&data, &c.id).unwrap().name, "B");
        c.name = "C".into();
        queue(data.clone(), c.clone());
        std::thread::sleep(QUIET + Duration::from_millis(400));
        assert_eq!(store::load(&data, &c.id).unwrap().name, "C", "written by the thread");
        c.name = "D".into();
        queue(data.clone(), c.clone());
        forget(&c.id);
        settle();
        assert_eq!(store::load(&data, &c.id).unwrap().name, "C", "a deleted company's change is not written");
        assert!(failed().is_empty());
        let _ = std::fs::remove_dir_all(&data);
    }
}
