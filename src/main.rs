use std::thread;

mod pipewire_connection;

use signal_hook::{
    consts::{SIGINT, SIGQUIT, SIGTERM},
    iterator::Signals,
};

fn main() {
    env_logger::init();

    let (pw_thread, pw_thread_terminate) = pipewire_connection::PWThread::new();

    let mut signals =
        Signals::new(&[SIGINT, SIGQUIT, SIGTERM]).expect("Failed to create signal listener");
    let signal_thread = thread::spawn(move || {
        for _sig in signals.wait() {
            pw_thread_terminate.send();
        }
    });

    pw_thread.run();
    signal_thread.join().unwrap();
}
