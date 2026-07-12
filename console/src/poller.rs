use std::{
    io, mem,
    net::SocketAddr,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use kameo::console::{Client, wire::Snapshot};

use crate::ConnectionState;

pub fn spawn_poller(
    addr: SocketAddr,
    interval: Arc<AtomicU64>,
    connection_timeout: Duration,
    auth_token: Option<Arc<str>>,
    snapshot: Arc<Mutex<Option<Snapshot>>>,
    connection_state: Arc<Mutex<ConnectionState>>,
) -> JoinHandle<()> {
    thread::spawn(move || {
        loop {
            let poller = connect_loop(
                &addr,
                connection_timeout,
                auth_token.as_deref(),
                &snapshot,
                &connection_state,
            );
            poll_loop(poller, &interval, &connection_state);
        }
    })
}

fn connect_loop(
    addr: &SocketAddr,
    connection_timeout: Duration,
    auth_token: Option<&str>,
    snapshot: &Arc<Mutex<Option<Snapshot>>>,
    connection_state: &Arc<Mutex<ConnectionState>>,
) -> Poller {
    loop {
        *connection_state.lock().unwrap() = ConnectionState::Connecting;
        match Poller::connect(addr, connection_timeout, auth_token, Arc::clone(snapshot)) {
            Ok(poller) => {
                *connection_state.lock().unwrap() = ConnectionState::Connected;
                return poller;
            }
            Err(err) => {
                *connection_state.lock().unwrap() = ConnectionState::Disconnected {
                    error: format!("{err}"),
                    since: Instant::now(),
                };
                thread::sleep(Duration::from_secs(5));
            }
        }
    }
}

fn poll_loop(
    mut poller: Poller,
    interval: &Arc<AtomicU64>,
    connection_state: &Arc<Mutex<ConnectionState>>,
) {
    loop {
        let start = Instant::now();
        match poller.poll() {
            Ok(()) => {
                let interval = Duration::from_millis(interval.load(Ordering::Relaxed));
                let sleep_duration = interval.saturating_sub(start.elapsed());
                thread::sleep(sleep_duration);
            }
            Err(err) => {
                *connection_state.lock().unwrap() = ConnectionState::Disconnected {
                    error: format!("{err}"),
                    since: Instant::now(),
                };
                mem::drop(poller);
                return;
            }
        }
    }
}

struct Poller {
    runtime: tokio::runtime::Runtime,
    client: Client,
    snapshot: Arc<Mutex<Option<Snapshot>>>,
}

impl Poller {
    fn connect(
        addr: &SocketAddr,
        connection_timeout: Duration,
        auth_token: Option<&str>,
        snapshot: Arc<Mutex<Option<Snapshot>>>,
    ) -> io::Result<Self> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        let client = runtime.block_on(Client::connect(
            *addr,
            connection_timeout,
            auth_token.map(str::as_bytes),
        ))?;
        Ok(Poller {
            runtime,
            client,
            snapshot,
        })
    }

    fn poll(&mut self) -> io::Result<()> {
        let snapshot = self.runtime.block_on(self.client.snapshot())?;
        *self.snapshot.lock().unwrap() = Some(snapshot);

        Ok(())
    }
}
