//! Opening a port on the home router without anyone logging into it.
//!
//! Most home routers answer UPnP: a device on the network asks the router, in
//! plain HTTP, to forward a port to it. This is the small part of that
//! conversation hosting needs: find the router, ask it to forward a UDP port,
//! ask it what the network's public address is, and take the forward away
//! again. Carrier networks (a phone on mobile data) and routers with UPnP
//! switched off do not answer, and then a port has to be forwarded by hand.

use std::io::{Read, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpStream, UdpSocket};
use std::time::Duration;

const SSDP: &str = "239.255.255.250:1900";
const TIMEOUT: Duration = Duration::from_secs(3);

/// The router's forwarding service: where to send requests and which kind it is.
#[derive(Clone, Debug)]
pub struct Gateway {
    host: SocketAddr,
    control: String,
    service: String,
}

/// Look for the router for a few seconds.
pub fn discover() -> Result<Gateway, String> {
    let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)).map_err(|e| e.to_string())?;
    socket.set_read_timeout(Some(Duration::from_millis(700))).ok();
    for target in [
        "urn:schemas-upnp-org:device:InternetGatewayDevice:1",
        "urn:schemas-upnp-org:service:WANIPConnection:1",
    ] {
        let request = format!(
            "M-SEARCH * HTTP/1.1\r\nHOST: {SSDP}\r\nMAN: \"ssdp:discover\"\r\nMX: 2\r\nST: {target}\r\n\r\n"
        );
        let _ = socket.send_to(request.as_bytes(), SSDP);
    }
    let started = std::time::Instant::now();
    let mut packet = [0u8; 2048];
    while started.elapsed() < TIMEOUT {
        let Ok((length, _)) = socket.recv_from(&mut packet) else {
            continue;
        };
        let reply = String::from_utf8_lossy(&packet[..length]);
        let Some(location) = header(&reply, "location") else {
            continue;
        };
        if let Ok(gateway) = describe(location.trim()) {
            return Ok(gateway);
        }
    }
    Err("the router did not answer (UPnP may be off)".to_owned())
}

fn header<'a>(text: &'a str, name: &str) -> Option<&'a str> {
    text.lines().find_map(|line| {
        let (key, value) = line.split_once(':')?;
        key.trim().eq_ignore_ascii_case(name).then_some(value.trim())
    })
}

/// `http://192.168.1.1:5000/desc.xml` -> (address, path).
fn split_url(url: &str) -> Result<(SocketAddr, String), String> {
    let rest = url.strip_prefix("http://").ok_or("not an http address")?;
    let (authority, path) = rest.split_once('/').map_or((rest, ""), |(a, p)| (a, p));
    let authority = if authority.contains(':') {
        authority.to_owned()
    } else {
        format!("{authority}:80")
    };
    let address = authority
        .parse::<SocketAddr>()
        .map_err(|e| format!("router address: {e}"))?;
    Ok((address, format!("/{path}")))
}

fn http(
    host: SocketAddr,
    method: &str,
    path: &str,
    extra: &str,
    body: &str,
) -> Result<(u16, String), String> {
    let mut stream = TcpStream::connect_timeout(&host, TIMEOUT).map_err(|e| e.to_string())?;
    stream.set_read_timeout(Some(TIMEOUT)).ok();
    stream.set_write_timeout(Some(TIMEOUT)).ok();
    let request = format!(
        "{method} {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n{extra}Content-Length: {}\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(request.as_bytes()).map_err(|e| e.to_string())?;
    let mut reply = Vec::new();
    let _ = stream.read_to_end(&mut reply);
    let reply = String::from_utf8_lossy(&reply).into_owned();
    let status = reply
        .split_whitespace()
        .nth(1)
        .and_then(|code| code.parse::<u16>().ok())
        .ok_or("no answer from the router")?;
    let body = reply.split_once("\r\n\r\n").map_or("", |(_, b)| b).to_owned();
    Ok((status, body))
}

fn tag<'a>(text: &'a str, name: &str) -> Option<&'a str> {
    let open = format!("<{name}>");
    let close = format!("</{name}>");
    let start = text.find(&open)? + open.len();
    let end = text[start..].find(&close)? + start;
    Some(text[start..end].trim())
}

fn describe(location: &str) -> Result<Gateway, String> {
    let (host, path) = split_url(location)?;
    let (status, xml) = http(host, "GET", &path, "", "")?;
    if status != 200 {
        return Err(format!("router description: HTTP {status}"));
    }
    // Each <service> block names a type and a control address.
    for block in xml.split("<service>").skip(1) {
        let Some(service) = tag(block, "serviceType") else {
            continue;
        };
        if !(service.contains("WANIPConnection") || service.contains("WANPPPConnection")) {
            continue;
        }
        let Some(control) = tag(block, "controlURL") else {
            continue;
        };
        let control = if control.starts_with('/') {
            control.to_owned()
        } else {
            format!("/{control}")
        };
        return Ok(Gateway {
            host,
            control,
            service: service.to_owned(),
        });
    }
    Err("the router offers no port forwarding service".to_owned())
}

fn soap(gateway: &Gateway, action: &str, arguments: &str) -> Result<String, String> {
    let body = format!(
        "<?xml version=\"1.0\"?>\r\n<s:Envelope xmlns:s=\"http://schemas.xmlsoap.org/soap/envelope/\" s:encodingStyle=\"http://schemas.xmlsoap.org/soap/encoding/\"><s:Body><u:{action} xmlns:u=\"{service}\">{arguments}</u:{action}></s:Body></s:Envelope>",
        service = gateway.service
    );
    let extra = format!(
        "Content-Type: text/xml; charset=\"utf-8\"\r\nSOAPAction: \"{}#{action}\"\r\n",
        gateway.service
    );
    let (status, reply) = http(gateway.host, "POST", &gateway.control, &extra, &body)?;
    if status == 200 {
        Ok(reply)
    } else {
        let code = tag(&reply, "errorCode").unwrap_or("?");
        Err(format!("router refused ({action}: HTTP {status}, error {code})"))
    }
}

/// Forward `port` (UDP) on the router to this device for an hour. Call again
/// before the hour is up to keep it.
pub fn map_udp(gateway: &Gateway, port: u16, internal: Ipv4Addr) -> Result<(), String> {
    let arguments = format!(
        "<NewRemoteHost></NewRemoteHost><NewExternalPort>{port}</NewExternalPort><NewProtocol>UDP</NewProtocol><NewInternalPort>{port}</NewInternalPort><NewInternalClient>{internal}</NewInternalClient><NewEnabled>1</NewEnabled><NewPortMappingDescription>IW4LJ</NewPortMappingDescription><NewLeaseDuration>3600</NewLeaseDuration>"
    );
    soap(gateway, "AddPortMapping", &arguments).map(|_| ())
}

pub fn unmap_udp(gateway: &Gateway, port: u16) {
    let arguments = format!(
        "<NewRemoteHost></NewRemoteHost><NewExternalPort>{port}</NewExternalPort><NewProtocol>UDP</NewProtocol>"
    );
    let _ = soap(gateway, "DeletePortMapping", &arguments);
}

/// The address the router has on the internet.
pub fn external_address(gateway: &Gateway) -> Result<Ipv4Addr, String> {
    let reply = soap(gateway, "GetExternalIPAddress", "")?;
    tag(&reply, "NewExternalIPAddress")
        .and_then(|text| text.parse().ok())
        .ok_or_else(|| "the router did not say its address".to_owned())
}

/// Whether an address is one only a private network can reach: a router that
/// reports one has another router (the carrier's) in front of it.
pub fn is_private(address: Ipv4Addr) -> bool {
    let octets = address.octets();
    address.is_private()
        || address.is_loopback()
        || address.is_link_local()
        || (octets[0] == 100 && (64..128).contains(&octets[1]))
        || address.is_unspecified()
}

#[cfg(test)]
mod tests {
    /// Read-only probe of the real router: finds it and asks for its address.
    /// Run by hand: `cargo test -p lan -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn probe_router() {
        let gateway = super::discover();
        println!("gateway: {gateway:?}");
        if let Ok(gateway) = gateway {
            println!("external: {:?}", super::external_address(&gateway));
        }
    }
}
