use crossbeam_channel::Sender;
use git2::{
    build::{CheckoutBuilder, RepoBuilder},
    FetchOptions, RemoteCallbacks, Config, ConfigLevel,
};
pub fn download_repo(path: &str, uri: &str, sender: &mut Sender<(String, String, usize, usize)>) {
    let mut co = CheckoutBuilder::new();
    co.progress(|_, cur, total| {
        sender.send(("co".to_string(), uri.to_string(), cur, total)).unwrap();
    });

    let mut cb = RemoteCallbacks::new();
    cb.transfer_progress(|stats| {
        let deltas = stats.indexed_deltas();
        if deltas > 0 {
            sender.send(("do".to_string(), uri.to_string(), stats.indexed_deltas(), stats.total_deltas())).unwrap();
        } else {
            sender.send(("fo".to_string(), uri.to_string(), stats.received_objects(), stats.total_objects())).unwrap();
        }
        true
    });

    let mut fo = FetchOptions::new();
    fo.remote_callbacks(cb);

    RepoBuilder::new()
        .with_checkout(co)
        .fetch_options(fo)
        .clone(uri, std::path::Path::new(path))
        .expect(&format!("Could not clone {}", uri));

}
