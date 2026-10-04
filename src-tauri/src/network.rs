//! This computer's address on the local network, for "point your POS at…".

use std::net::{IpAddr, Ipv4Addr, UdpSocket};

/// The IPv4 address the OS would use to reach other machines, or `None` offline.
///
/// "Connecting" a UDP socket sends nothing: it only asks the OS to pick the outgoing
/// interface. 192.0.2.1 is TEST-NET-1 (RFC 5737), reserved for documentation, so no real
/// host is involved either way.
pub fn lan_ipv4() -> Option<Ipv4Addr> {
    let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)).ok()?;
    socket.connect((Ipv4Addr::new(192, 0, 2, 1), 9)).ok()?;
    match socket.local_addr().ok()?.ip() {
        IpAddr::V4(ip) if !ip.is_loopback() && !ip.is_unspecified() => Some(ip),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::lan_ipv4;

    #[test]
    fn is_a_usable_address_when_present() {
        // Offline CI runners may have none; when there is one it must be routable.
        if let Some(ip) = lan_ipv4() {
            assert!(!ip.is_loopback() && !ip.is_unspecified() && !ip.is_multicast());
        }
    }
}
