use crossbeam_channel::Sender;
use git2::{
    build::{CheckoutBuilder, RepoBuilder},
    Config, ConfigLevel, FetchOptions, RemoteCallbacks,
};

pub enum GitPhase {
    Fetch(usize, usize),
    DeltaResolve(usize, usize),
    Checkout(usize, usize),
}

pub fn download_repo<T: AsRef<str> + ToString>(path: T, uri: T, sender: &mut Sender<(GitPhase, String)>) {
    let mut co = CheckoutBuilder::new();
    co.progress(|_, cur, total| {
        sender
            .send((GitPhase::Checkout(cur, total), uri.to_string()))
            .unwrap();
    });

    let mut cb = RemoteCallbacks::new();
    cb.transfer_progress(|stats| {
        let deltas = stats.indexed_deltas();
        if deltas > 0 {
            sender
                .send((
                    GitPhase::DeltaResolve(stats.indexed_deltas(), stats.total_deltas()),
                    uri.to_string(),
                ))
                .unwrap();
        } else {
            sender
                .send((
                    GitPhase::Fetch(stats.received_objects(), stats.total_objects()),
                    uri.to_string(),
                ))
                .unwrap();
        }
        true
    });

    let mut fo = FetchOptions::new();
    fo.remote_callbacks(cb);

    RepoBuilder::new()
        .with_checkout(co)
        .fetch_options(fo)
        .clone(uri.as_ref(), std::path::Path::new(path.as_ref()))
        .expect(&format!("Could not clone {}", uri.as_ref()));
}
