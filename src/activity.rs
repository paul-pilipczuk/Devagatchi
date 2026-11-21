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
    pub errors: Vec<String>,
}

// Prepare a repository's entire bounded batch before changing pet state. A bad
// repository cannot erase successful work from another global connection.
struct Batch {
    events: Vec<Event>,
    head: String,
    pending: bool,
}

pub fn sync(pet: &mut Pet, only_root: Option<&str>, cache: &mut Cache) -> Report {
    sync_repositories(pet, only_root, cache, false)
}

pub fn catch_up(pet: &mut Pet, only_root: Option<&str>, cache: &mut Cache) -> Report {
    sync_repositories(pet, only_root, cache, true)
}

fn sync_repositories(
    pet: &mut Pet,
    only_root: Option<&str>,
    cache: &mut Cache,
    catch_up: bool,
) -> Report {
    let mut report = Report::default();
    for index in 0..pet.repos.len() {
        if only_root.is_some_and(|r| r != pet.repos[index].root) {
            continue;
        }
        match prepare(pet, index, cache, catch_up) {
            Ok(batch) => {
                for event in batch.events {
                    report.commits += 1;
                    report.added = report.added.saturating_add(event.added);
                    report.deleted = report.deleted.saturating_add(event.deleted);
                    pet.seen.insert(event.key.clone());
                    pet.events.push(event);
                }
                pet.repos[index].head = batch.head;
                report.pending |= batch.pending;
            }
            Err(error) => report
                .errors
                .push(format!("{}: {error}", pet.repos[index].root)),
        }
    }
    report
}

fn prepare(pet: &Pet, index: usize, cache: &mut Cache, catch_up: bool) -> Result<Batch, String> {
    let repo = &pet.repos[index];
    let head = git::head(&repo.root)?;
    if head.is_empty() || head == repo.head {
        return Ok(Batch {
            events: vec![],
            head: repo.head.clone(),
            pending: false,
        });
    }
    if !repo.head.is_empty() {
        git::run(&repo.root, &["merge-base", "--is-ancestor", &repo.head, &head])
            .map_err(|_| "History changed or was rewritten. No automatic reward: use `devagatchi reconcile` in that checkout to baseline current history without feeding.".to_string())?;
    }
    let exclusion = format!("^{}", repo.head);
    let limit = format!("--max-count={}", BATCH + 1);
    let mut args = vec!["rev-list", "--topo-order", "--reverse"];
    if !catch_up {
        args.push(&limit);
    }
    args.push(&head);
    if !repo.head.is_empty() {
        args.push(&exclusion);
    }
    // Explicit catch-up may read the ID list, subject to git::bytes' 16 MiB limit.
    // Only BATCH unseen commits have statistics computed on each invocation.
    let listing = git::run(&repo.root, &args)?;
    let all: Vec<_> = listing.lines().collect();
    if !catch_up && all.len() > BATCH {
        return Err(format!("More than {BATCH} pending commits. Use `devagatchi sync --catch-up`, or reconcile to baseline without feeding."));
    }
    let ids: Vec<_> = all
        .into_iter()
        .filter(|id| !pet.seen.contains(&format!("{}:{id}", repo.common)))
        .collect();
    let mut events = vec![];
    for id in ids.iter().take(BATCH) {
        let key = format!("{}:{id}", repo.common);
        let cache_key = format!("{}:{:?}", key, pet.excludes);
        let (added, deleted, timestamp) = match cache.stats.get(&cache_key) {
            Some(stats) => *stats,
            None => {
                let stats = git::stats(&repo.root, id, &pet.excludes)?;
                cache.stats.insert(cache_key, stats);
                stats
            }
        };
        events.push(Event {
            key,
            at: timestamp.max(repo.since).min(now()),
            added,
            deleted,
            kind: "c".into(),
        });
    }
    let pending = ids.len() > BATCH;
    Ok(Batch {
        events,
        head: if pending { repo.head.clone() } else { head },
        pending,
    })
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
