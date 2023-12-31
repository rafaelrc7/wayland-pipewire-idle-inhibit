use std::thread;

mod pipewire_connection;
mod wayland_connection;
use wayland_connection::WaylandConnection;

use signal_hook::{
    consts::{SIGINT, SIGQUIT, SIGTERM},
    iterator::Signals,
};

fn main() {
    env_logger::init();

    let mut wayland_connection = WaylandConnection::new();
    let (pw_thread, pw_thread_terminate) = pipewire_connection::PWThread::new();

    let mut signals =
        Signals::new(&[SIGINT, SIGQUIT, SIGTERM]).expect("Failed to create signal listener");
    let signal_thread = thread::spawn(move || {
        for _sig in signals.wait() {
            pw_thread_terminate.send();
        }
    });

    pw_thread.run(move |inhibit_idle| wayland_connection.set_inhibit_idle(inhibit_idle));
    signal_thread.join().unwrap();
}
