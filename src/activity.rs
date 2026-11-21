use crate::{
    git, now,
    state::{Event, Pet},
};
use std::collections::HashMap;

pub const BATCH: usize = 200;
#[derive(Default)]
pub struct Cache {
    stats: HashMap<String, (u64, u64, u64)>,
}
#[derive(Default)]
pub struct Report {
    pub commits: u64,
    pub added: u64,
    pub deleted: u64,
    pub pending: bool,
}

// Same function for hooks and explicit sync. Per-pet locking is held by the caller.
pub fn sync(pet: &mut Pet, only_root: Option<&str>, cache: &mut Cache) -> Result<Report, String> {
    let mut report = Report::default();
    for index in 0..pet.repos.len() {
        let repo = pet.repos[index].clone();
        if only_root.is_some_and(|r| r != repo.root) {
            continue;
        }
        let head = git::head(&repo.root)?;
        if head.is_empty() || head == repo.head {
            continue;
        }
        if !repo.head.is_empty() {
            git::run(&repo.root,&["merge-base","--is-ancestor",&repo.head,&head]).map_err(|_|"History changed or was rewritten. No automatic reward: run `devagatchi reconcile` in that checkout to baseline its current history without feeding.".to_string())?;
        }
        // Bounded traversal: collect newest BATCH+1 entries, then process oldest-first.
        // When a history is bigger, stop before changing state; explicit sync can choose
        // a baseline via reconcile. This avoids silently skipping ancestors.
        let exclusion = format!("^{}", repo.head);
        let limit = format!("--max-count={}", BATCH + 1);
        let mut args = vec!["rev-list", "--topo-order", "--reverse", &limit, &head];
        if !repo.head.is_empty() {
            args.push(&exclusion);
        }
        let commits = git::run(&repo.root, &args)?;
        let ids: Vec<_> = commits.lines().collect();
        if ids.len() > BATCH {
            return Err(format!("More than {BATCH} pending commits. Use `devagatchi sync --catch-up` for bounded batches, or reconcile to baseline without feeding."));
        }
        ingest(pet, index, &ids, cache, &mut report)?;
        pet.repos[index].head = head;
    }
    Ok(report)
}
fn ingest(
    pet: &mut Pet,
    index: usize,
    ids: &[&str],
    cache: &mut Cache,
    report: &mut Report,
) -> Result<(), String> {
    let repo = pet.repos[index].clone();
    for id in ids {
        let key = format!("{}:{id}", repo.common);
        if pet.seen.contains(&key) {
            continue;
        }
        let cache_key = format!("{}:{:?}", key, pet.excludes);
        let (added, deleted, timestamp) = if let Some(s) = cache.stats.get(&cache_key) {
            *s
        } else {
            let s = git::stats(&repo.root, id, &pet.excludes)?;
            cache.stats.insert(cache_key, s);
            s
        };
        // Historical meals decay from the commit time. Clamp untrusted clocks to enrollment/now.
        let at = timestamp.max(repo.since).min(now());
        pet.events.push(Event {
            key: key.clone(),
            at,
            added,
            deleted,
            kind: "c".into(),
        });
        pet.seen.insert(key);
        report.commits += 1;
        report.added = report.added.saturating_add(added);
        report.deleted = report.deleted.saturating_add(deleted);
    }
    Ok(())
}

// A bounded batch uses the full list of IDs (no source/patches) only when requested.
// git::bytes has a hard output limit; never silently process a truncated list.
pub fn catch_up(
    pet: &mut Pet,
    only_root: Option<&str>,
    cache: &mut Cache,
) -> Result<Report, String> {
    let mut report = Report::default();
    for index in 0..pet.repos.len() {
        let repo = pet.repos[index].clone();
        if only_root.is_some_and(|r| r != repo.root) {
            continue;
        }
        let head = git::head(&repo.root)?;
        if head.is_empty() || head == repo.head {
            continue;
        }
        if !repo.head.is_empty() {
            git::run(
                &repo.root,
                &["merge-base", "--is-ancestor", &repo.head, &head],
            )
            .map_err(|_| {
                "Rewritten history: use reconcile to baseline without feeding".to_string()
            })?;
        }
        let exclusion = format!("^{}", repo.head);
        let mut args = vec!["rev-list", "--topo-order", "--reverse", &head];
        if !repo.head.is_empty() {
            args.push(&exclusion);
        }
        let listing = git::run(&repo.root, &args)?;
        let ids: Vec<_> = listing
            .lines()
            .filter(|id| !pet.seen.contains(&format!("{}:{id}", repo.common)))
            .collect();
        let batch = &ids[..ids.len().min(BATCH)];
        ingest(pet, index, batch, cache, &mut report)?;
        if ids.len() <= BATCH {
            pet.repos[index].head = head;
        } else {
            report.pending = true;
        }
        // Keep the original frontier until every branch in the range was processed.
        // Seen IDs make subsequent calls skip completed work.
    }
    Ok(report)
}

pub fn rewrite(pet: &mut Pet, root: &str, mappings: &[(String, String)]) -> Result<(), String> {
    for repo in &mut pet.repos {
        if repo.root != root {
            continue;
        }
        for (old, new) in mappings {
            let old_key = format!("{}:{old}", repo.common);
            let new_key = format!("{}:{new}", repo.common);
            if pet.seen.contains(&old_key) || *old == repo.head || *old == repo.baseline {
                pet.seen.insert(new_key);
            }
            if *old == repo.baseline {
                repo.baseline = new.clone();
            }
            if *old == repo.head {
                repo.head = new.clone();
            }
        }
    }
    Ok(())
}
