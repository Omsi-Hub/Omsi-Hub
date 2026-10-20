//! A line the Timeplan saved, written into the map on a thread of its own (`timeplan::save`,
//! `timeplan::commit`): the registry, the map's timetable and the depot files take seconds on a
//! big map, and every other command waited for them under the company's session. The writes go
//! one after the other in the order they were asked for; once one is done the company's
//! timetables are read again and what failed is said on the page (`timeplan::written`). What
//! reads a registry with a write on its way waits for it first (`settle`).

use std::path::{Path, PathBuf};
use std::sync::{Condvar, Mutex, MutexGuard, OnceLock};

#[derive(Default)]
struct Queue {
    /// The tickets given, and the last written.
    given: u64,
    done: u64,
    /// The registries a write is on its way to, by ticket.
    waiting: Vec<(u64, PathBuf)>,
}

struct Writes {
    q: Mutex<Queue>,
    cv: Condvar,
}

fn writes() -> &'static Writes {
    static W: OnceLock<Writes> = OnceLock::new();
    W.get_or_init(|| Writes { q: Mutex::new(Queue::default()), cv: Condvar::new() })
}

fn lock() -> MutexGuard<'static, Queue> {
    writes().q.lock().unwrap_or_else(|e| e.into_inner())
}

/// `write` (into registry `path`) done after the writes asked for before it, then `after` with
/// what it gave - on a thread of its own (the website: after the command).
pub(crate) fn queue<T: Send + 'static>(path: PathBuf, write: impl FnOnce() -> T + Send + 'static, after: impl FnOnce(T) + Send + 'static) {
    let ticket = {
        let mut q = lock();
        q.given += 1;
        let t = q.given;
        q.waiting.push((t, path));
        t
    };
    crate::spawn(move || {
        let w = writes();
        {
            let mut q = lock();
            while q.done + 1 < ticket {
                q = w.cv.wait(q).unwrap_or_else(|e| e.into_inner());
            }
        }
        let out = write();
        {
            let mut q = lock();
            q.done = ticket;
            q.waiting.retain(|x| x.0 != ticket);
            w.cv.notify_all();
        }
        after(out);
    });
}

/// Until no write is on its way to registry `path` (every registry: None).
pub(crate) fn settle(path: Option<&Path>) {
    // (the website: the writes run after the command, none is under way meanwhile)
    if !cfg!(feature = "native") {
        return;
    }
    let w = writes();
    let mut q = lock();
    while q.waiting.iter().any(|x| path.is_none_or(|p| x.1 == p)) {
        q = w.cv.wait(q).unwrap_or_else(|e| e.into_inner());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn writes_go_in_their_order_and_settle_waits_for_them() {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let path = PathBuf::from("writes-test-registry.json");
        for i in 0..5 {
            let (a, b) = (seen.clone(), seen.clone());
            queue(
                path.clone(),
                move || {
                    std::thread::sleep(std::time::Duration::from_millis(if i == 0 { 120 } else { 5 }));
                    a.lock().unwrap().push(i);
                    i
                },
                move |x| b.lock().unwrap().push(100 + x),
            );
        }
        settle(Some(&path));
        let written: Vec<i32> = seen.lock().unwrap().iter().copied().filter(|x| *x < 100).collect();
        assert_eq!(written, vec![0, 1, 2, 3, 4]);
    }
}
