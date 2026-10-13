//! LAN play with no server.
//!
//! The runtime has one way for two players to meet: both connect to a master
//! (a QUIC relay) named by a community descriptor, and the relay carries the
//! match. On a LAN there is no master, so one of the players becomes it:
//!
//! - every device is given the built-in [`descriptor`], whose master lives at
//!   `127.0.0.1` — its own loopback;
//! - the **host** runs the relay in-process on that port ([`host`]) and
//!   announces itself with a UDP broadcast once a second;
//! - a **joiner** listens for the announcement ([`join`]) and then forwards its
//!   own loopback port to the host's, so the same descriptor reaches the
//!   host's relay without the game being told an address.
//!
//! Nothing here is secret or authenticated beyond that: the relay's
//! certificate and key are compiled in so that every copy of the game trusts
//! every other, which is the right amount of trust for a living room.

mod upnp;

use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// The relay's port, on every device's loopback and on the host's LAN address.
pub const RELAY_PORT: u16 = 28960;
/// The port the host announces itself on.
pub const BEACON_PORT: u16 = 28961;
/// The name the relay's certificate is issued to.
pub const SERVER_NAME: &str = "iw4l-lan";

const CA_PEM: &str = include_str!("../certs/ca.pem");
const CERT_PEM: &str = include_str!("../certs/server-cert.pem");
const KEY_PEM: &str = include_str!("../certs/server-key.pem");
const BEACON: &[u8] = b"IW4LJ-LAN/1";
/// How long a joiner listens before giving up.
const SEARCH: Duration = Duration::from_secs(20);

/// The community descriptor that points a device at its own loopback relay.
pub fn descriptor() -> String {
    format!(
        "schema = 1\nname = \"LAN\"\n\n[master]\naddress = \"127.0.0.1:{RELAY_PORT}\"\nserver_name = \"{SERVER_NAME}\"\n\n[updates]\nurl = \"https://127.0.0.1:{RELAY_PORT}/updates/manifest.toml\"\nca_pem = \"\"\"\n{CA_PEM}\"\"\"\n"
    )
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Status {
    Idle,
    /// This device is the relay; others on the network can find it.
    Hosting,
    /// Listening for a host's announcement.
    Searching,
    /// Forwarding to the host at this address.
    Joined(IpAddr),
    Failed(String),
}

impl Status {
    /// One line for a menu.
    pub fn line(&self) -> String {
        match self {
            Self::Idle => "LAN: off".to_owned(),
            Self::Hosting => {
                let online = ONLINE.lock().map(|text| text.clone()).unwrap_or_default();
                if !online.is_empty() {
                    return online;
                }
                match local_addresses().first() {
                    Some(address) => format!(
                        "LAN: hosting at {address} — friends on this Wi-Fi can join (JOIN BY IP if they cannot find it)"
                    ),
                    None => "LAN: hosting — friends on this Wi-Fi can join".to_owned(),
                }
            }
            Self::Searching => "LAN: looking for a host on this Wi-Fi…".to_owned(),
            Self::Joined(host) => format!("LAN: found host {host}"),
            Self::Failed(error) => format!("LAN: {error}"),
        }
    }
}

struct State {
    status: Status,
    /// The relay's runtime, kept alive while hosting.
    relay: Option<tokio::runtime::Runtime>,
}

/// What hosting online has found out: the address to give friends, and whether
/// the router's port was opened. Empty for a plain LAN host.
static ONLINE: Mutex<String> = Mutex::new(String::new());

fn set_online(generation: u64, text: String) {
    if current(generation)
        && let Ok(mut online) = ONLINE.lock()
    {
        *online = text;
    }
}

static STATE: Mutex<State> = Mutex::new(State {
    status: Status::Idle,
    relay: None,
});
/// Bumped by every [`host`], [`join`] and [`stop`]; a worker thread exits when
/// it is no longer the generation it was started for.
static GENERATION: AtomicU64 = AtomicU64::new(0);

fn current(generation: u64) -> bool {
    GENERATION.load(Ordering::Relaxed) == generation
}

fn set_status(generation: u64, status: Status) {
    if current(generation)
        && let Ok(mut state) = STATE.lock()
    {
        state.status = status;
    }
}

pub fn status() -> Status {
    STATE
        .lock()
        .map_or(Status::Idle, |state| state.status.clone())
}

/// Stop hosting or forwarding.
pub fn stop() {
    begin(Status::Idle);
}

/// Retire whatever was running and return the new generation.
fn begin(status: Status) -> u64 {
    let generation = GENERATION.fetch_add(1, Ordering::Relaxed) + 1;
    if let Ok(mut state) = STATE.lock() {
        if let Some(relay) = state.relay.take() {
            relay.shutdown_background();
        }
        state.status = status;
    }
    if let Ok(mut online) = ONLINE.lock() {
        online.clear();
    }
    generation
}

/// Become the relay for this network and announce it.
pub fn host() {
    if status() == Status::Hosting {
        return;
    }
    let generation = begin(Status::Hosting);
    // A forwarder from an earlier `join` holds the loopback port for at most
    // one receive timeout after its generation ends.
    std::thread::sleep(FORWARD_POLL * 2);

    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .thread_name("lan-relay")
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => {
            set_status(generation, Status::Failed(format!("relay runtime: {error}")));
            return;
        }
    };
    runtime.spawn(async move {
        let bind = SocketAddr::from((Ipv4Addr::UNSPECIFIED, RELAY_PORT));
        if let Err(error) =
            iw4l_master::serve_pem(bind, CERT_PEM.as_bytes(), KEY_PEM.as_bytes()).await
        {
            set_status(generation, Status::Failed(format!("relay: {error}")));
        }
    });
    if let Ok(mut state) = STATE.lock()
        && current(generation)
    {
        state.relay = Some(runtime);
    }

    let _ = std::thread::Builder::new()
        .name("lan-beacon".into())
        .spawn(move || {
            let Ok(socket) = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)) else {
                return;
            };
            let _ = socket.set_broadcast(true);
            while current(generation) {
                // The limited broadcast goes out of one interface of the
                // system's choosing, and some routers and phones drop it; a
                // broadcast to the network's own address reaches the Wi-Fi
                // however many interfaces the device has.
                let _ = socket.send_to(BEACON, (Ipv4Addr::BROADCAST, BEACON_PORT));
                for (_, broadcast) in interfaces() {
                    let _ = socket.send_to(BEACON, (broadcast, BEACON_PORT));
                }
                std::thread::sleep(Duration::from_secs(1));
            }
        });
}

/// Host for players outside this network too, who join by this phone's public
/// address. The router is asked to forward the relay's port; when it will not,
/// the player has to forward it by hand, and the status line says so.
pub fn host_online() {
    host();
    let generation = GENERATION.load(Ordering::Relaxed);
    set_online(generation, "ONLINE: opening the port on your router…".to_owned());
    let _ = std::thread::Builder::new()
        .name("lan-upnp".into())
        .spawn(move || online_worker(generation));
}

fn online_worker(generation: u64) {
    let Some(internal) = local_addresses().first().copied() else {
        set_online(generation, "ONLINE: no Wi-Fi address — connect to Wi-Fi to host by IP".to_owned());
        return;
    };
    let gateway = match upnp::discover() {
        Ok(gateway) => gateway,
        Err(error) => {
            set_online(generation, format!(
                "ONLINE: {error}. Forward UDP port {RELAY_PORT} on your router to {internal}, then friends join by your public IP (search \"what is my ip\"). Or use Tailscale: no forwarding needed."
            ));
            return;
        }
    };
    let mut announced = false;
    while current(generation) {
        match upnp::map_udp(&gateway, RELAY_PORT, internal) {
            Ok(()) if !announced => {
                announced = true;
                let message = match upnp::external_address(&gateway) {
                    Ok(address) if upnp::is_private(address) => format!(
                        "ONLINE: port opened, but your network sits behind carrier NAT ({address}) so outsiders cannot reach it. Use Tailscale, or host from a home Wi-Fi."
                    ),
                    Ok(address) => format!(
                        "ONLINE: friends join by IP {address} (port {RELAY_PORT} opened automatically)"
                    ),
                    Err(_) => format!(
                        "ONLINE: port {RELAY_PORT} opened; find your public IP (search \"what is my ip\") and give it to friends"
                    ),
                };
                set_online(generation, message);
            }
            Ok(()) => {}
            Err(error) => {
                if !announced {
                    announced = true;
                    set_online(generation, format!(
                        "ONLINE: {error}. Forward UDP port {RELAY_PORT} on your router to {internal} by hand, then friends join by your public IP."
                    ));
                }
            }
        }
        // The forward is leased for an hour; renew it halfway, in small steps so
        // a stopped host notices.
        for _ in 0..1800 {
            if !current(generation) {
                break;
            }
            std::thread::sleep(Duration::from_secs(1));
        }
    }
    upnp::unmap_udp(&gateway, RELAY_PORT);
}

/// Find a host on this network and forward the loopback relay port to it.
pub fn join() {
    if matches!(status(), Status::Searching | Status::Joined(_)) {
        return;
    }
    let generation = begin(Status::Searching);
    let _ = std::thread::Builder::new()
        .name("lan-join".into())
        .spawn(move || match find_host(generation) {
            Ok(host) => {
                set_status(generation, Status::Joined(host));
                if let Err(error) = forward(generation, host, RELAY_PORT) {
                    set_status(generation, Status::Failed(error));
                }
            }
            Err(error) => set_status(generation, Status::Failed(error)),
        });
}

/// Forward to a host whose address the player typed, skipping the search.
pub fn join_at(host: Ipv4Addr, port: u16) {
    let generation = begin(Status::Joined(IpAddr::V4(host)));
    let _ = std::thread::Builder::new()
        .name("lan-join".into())
        .spawn(move || {
            if let Err(error) = forward(generation, IpAddr::V4(host), port) {
                set_status(generation, Status::Failed(error));
            }
        });
}

/// This device's IPv4 addresses on real networks (not loopback), for showing
/// the host's address to a friend.
pub fn local_addresses() -> Vec<Ipv4Addr> {
    interfaces().into_iter().map(|(address, _)| address).collect()
}

/// Every up, broadcast-capable, non-loopback IPv4 interface as (address,
/// broadcast address).
#[cfg(unix)]
fn interfaces() -> Vec<(Ipv4Addr, Ipv4Addr)> {
    let mut found = Vec::new();
    // SAFETY: `getifaddrs` fills a list that is read only until `freeifaddrs`;
    // every pointer is checked for null and the address family before it is
    // read as an IPv4 socket address.
    unsafe {
        let mut list: *mut libc::ifaddrs = std::ptr::null_mut();
        if libc::getifaddrs(&raw mut list) != 0 {
            return found;
        }
        let mut at = list;
        while !at.is_null() {
            let entry = &*at;
            let flags = entry.ifa_flags;
            let usable = flags & libc::IFF_UP as u32 != 0
                && flags & libc::IFF_BROADCAST as u32 != 0
                && flags & libc::IFF_LOOPBACK as u32 == 0;
            if usable
                && !entry.ifa_addr.is_null()
                && !entry.ifa_netmask.is_null()
                && i32::from((*entry.ifa_addr).sa_family) == libc::AF_INET
            {
                let address = u32::from_be((*entry.ifa_addr.cast::<libc::sockaddr_in>()).sin_addr.s_addr);
                let mask = u32::from_be((*entry.ifa_netmask.cast::<libc::sockaddr_in>()).sin_addr.s_addr);
                found.push((Ipv4Addr::from(address), Ipv4Addr::from(address | !mask)));
            }
            at = entry.ifa_next;
        }
        libc::freeifaddrs(list);
    }
    found
}

#[cfg(not(unix))]
fn interfaces() -> Vec<(Ipv4Addr, Ipv4Addr)> {
    Vec::new()
}

fn find_host(generation: u64) -> Result<IpAddr, String> {
    let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, BEACON_PORT))
        .map_err(|error| format!("cannot listen for a host: {error}"))?;
    socket
        .set_read_timeout(Some(Duration::from_millis(500)))
        .map_err(|error| error.to_string())?;
    let started = Instant::now();
    let mut packet = [0u8; 64];
    while current(generation) && started.elapsed() < SEARCH {
        if let Ok((length, from)) = socket.recv_from(&mut packet)
            && packet[..length].starts_with(BEACON)
        {
            return Ok(from.ip());
        }
    }
    Err("no host found — is the other phone hosting on the same Wi-Fi?".to_owned())
}

/// How often a forwarding thread looks up from its socket to see whether it
/// has been retired.
const FORWARD_POLL: Duration = Duration::from_millis(300);

/// Carry datagrams between the game's connection to `127.0.0.1` and the host.
/// QUIC does not care what address its packets travel through, only that they
/// arrive, so the game connects to "its own" relay and reaches the host's.
fn forward(generation: u64, host: IpAddr, port: u16) -> Result<(), String> {
    let local = UdpSocket::bind((Ipv4Addr::LOCALHOST, RELAY_PORT))
        .map_err(|error| format!("cannot open the local relay port: {error}"))?;
    let upstream = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)).map_err(|error| error.to_string())?;
    upstream
        .connect((host, port))
        .map_err(|error| format!("cannot reach the host: {error}"))?;
    for socket in [&local, &upstream] {
        socket
            .set_read_timeout(Some(FORWARD_POLL))
            .map_err(|error| error.to_string())?;
    }
    let local = Arc::new(local);
    let upstream = Arc::new(upstream);
    // Where the game's own endpoint is; it moves when the game reconnects.
    let game: Arc<Mutex<Option<SocketAddr>>> = Arc::new(Mutex::new(None));

    let down = {
        let (local, upstream, game) = (local.clone(), upstream.clone(), game.clone());
        std::thread::Builder::new()
            .name("lan-forward-down".into())
            .spawn(move || {
                let mut packet = vec![0u8; 65_536];
                while current(generation) {
                    let Ok(length) = upstream.recv(&mut packet) else {
                        continue;
                    };
                    if let Some(game) = game.lock().ok().and_then(|game| *game) {
                        let _ = local.send_to(&packet[..length], game);
                    }
                }
            })
            .map_err(|error| error.to_string())?
    };
    let mut packet = vec![0u8; 65_536];
    while current(generation) {
        let Ok((length, from)) = local.recv_from(&mut packet) else {
            continue;
        };
        if let Ok(mut game) = game.lock() {
            *game = Some(from);
        }
        let _ = upstream.send(&packet[..length]);
    }
    let _ = down.join();
    Ok(())
}
