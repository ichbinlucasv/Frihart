//! Address classes. Dual-stack. Block private redirects (Port Authority).

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HostClass {
    Loopback,
    LinkLocal,
    Private,
    Public,
    Name,
}

pub fn classify_host(host: &str) -> HostClass {
    let h = host.trim().trim_matches(|c| c == '[' || c == ']');
    if let Ok(ip) = h.parse::<IpAddr>() {
        return classify_ip(ip);
    }
    if h.eq_ignore_ascii_case("localhost") {
        return HostClass::Loopback;
    }
    HostClass::Name
}

pub fn classify_ip(ip: IpAddr) -> HostClass {
    match ip {
        IpAddr::V4(v4) => classify_v4(v4),
        IpAddr::V6(v6) => classify_v6(v6),
    }
}

fn classify_v4(ip: Ipv4Addr) -> HostClass {
    if ip.is_loopback() {
        return HostClass::Loopback;
    }
    if ip.is_link_local() || ip.is_unspecified() {
        return HostClass::LinkLocal;
    }
    if ip.is_private() || ip.is_documentation() || ip.is_broadcast() || ip.is_multicast() {
        return HostClass::Private;
    }
    let o = ip.octets();
    // 0.0.0.0/8 ("this host"), CGNAT, benchmarking, IETF protocol
    // assignments, and the reserved former Class E block are not
    // public destinations. A name that resolves to one of them is a
    // rebinding or metadata bounce, same as 10/8 or 169.254/16.
    if o[0] == 0
        || (o[0] == 100 && (o[1] & 0b1100_0000) == 64)
        || (o[0] == 192 && o[1] == 0 && o[2] == 0)
        || (o[0] == 198 && (o[1] == 18 || o[1] == 19))
        || o[0] >= 240
    {
        return HostClass::Private;
    }
    HostClass::Public
}

fn classify_v6(ip: Ipv6Addr) -> HostClass {
    if ip.is_loopback() {
        return HostClass::Loopback;
    }
    if ip.is_unicast_link_local() || ip.is_unspecified() {
        return HostClass::LinkLocal;
    }
    let s = ip.segments();
    if ip.is_multicast() {
        return HostClass::Private;
    }
    // fc00::/7 unique local, fec0::/10 deprecated site-local.
    if (s[0] & 0xfe00) == 0xfc00 || (s[0] & 0xffc0) == 0xfec0 {
        return HostClass::Private;
    }
    // 2001:db8::/32 documentation, 100::/64 discard-only,
    // 2001:2::/48 benchmarking.
    if (s[0] == 0x2001 && s[1] == 0x0db8)
        || (s[0] == 0x0100 && s[1] == 0 && s[2] == 0 && s[3] == 0)
        || (s[0] == 0x2001 && s[1] == 0x0002 && s[2] == 0)
    {
        return HostClass::Private;
    }
    // NAT64 and 6to4 carry an IPv4 address inside a public-looking IPv6
    // prefix. Classify that IPv4, or a metadata address slips through.
    if let Some(v4) = embedded_ipv4(ip) {
        return classify_v4(v4);
    }
    // IPv4-mapped (::ffff:a.b.c.d) and deprecated IPv4-compatible (::a.b.c.d).
    if let Some(v4) = ip.to_ipv4_mapped().or_else(|| ip.to_ipv4()) {
        return classify_v4(v4);
    }
    HostClass::Public
}

/// IPv4 carried in a transition prefix: NAT64 well-known `64:ff9b::/96`,
/// NAT64 local-use `64:ff9b:1::/48`, and 6to4 `2002::/16`.
fn embedded_ipv4(ip: Ipv6Addr) -> Option<Ipv4Addr> {
    let s = ip.segments();
    if s[0] == 0x0064 && s[1] == 0xff9b && s[2] == 0 && s[3] == 0 && s[4] == 0 && s[5] == 0 {
        return Some(Ipv4Addr::new(
            (s[6] >> 8) as u8,
            (s[6] & 0xff) as u8,
            (s[7] >> 8) as u8,
            (s[7] & 0xff) as u8,
        ));
    }
    if s[0] == 0x0064 && s[1] == 0xff9b && s[2] == 0x0001 {
        // RFC 6052 with a 48-bit prefix: 16 bits, the u octet, then 16 bits.
        return Some(Ipv4Addr::new(
            (s[3] >> 8) as u8,
            (s[3] & 0xff) as u8,
            (s[4] & 0xff) as u8,
            (s[5] >> 8) as u8,
        ));
    }
    if s[0] == 0x2002 {
        return Some(Ipv4Addr::new(
            (s[1] >> 8) as u8,
            (s[1] & 0xff) as u8,
            (s[2] >> 8) as u8,
            (s[2] & 0xff) as u8,
        ));
    }
    None
}

pub fn is_sensitive(host: &str) -> bool {
    !matches!(classify_host(host), HostClass::Public | HostClass::Name)
}

/// True when a public-name navigation is being bounced onto a private
/// or loopback address (SSRF / metadata / port-scan).
pub fn private_redirect(from_host: &str, to_host: &str) -> bool {
    let from = classify_host(from_host);
    let to = classify_host(to_host);
    matches!(from, HostClass::Name | HostClass::Public)
        && matches!(
            to,
            HostClass::Loopback | HostClass::LinkLocal | HostClass::Private
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v4_and_v6_classes() {
        assert_eq!(classify_host("127.0.0.1"), HostClass::Loopback);
        assert_eq!(classify_host("10.0.0.1"), HostClass::Private);
        assert_eq!(classify_host("192.168.1.1"), HostClass::Private);
        assert_eq!(classify_host("172.16.0.1"), HostClass::Private);
        assert_eq!(classify_host("169.254.169.254"), HostClass::LinkLocal);
        assert_eq!(classify_host("0.0.0.1"), HostClass::Private);
        assert_eq!(classify_host("100.64.0.1"), HostClass::Private);
        assert_eq!(classify_host("192.0.0.8"), HostClass::Private);
        assert_eq!(classify_host("224.0.0.1"), HostClass::Private);
        assert_eq!(classify_host("240.0.0.1"), HostClass::Private);
        assert_eq!(classify_host("198.18.0.1"), HostClass::Private);
        assert_eq!(classify_host("100.127.255.254"), HostClass::Private);
        assert_eq!(classify_host("100.128.0.1"), HostClass::Public);
        assert_eq!(classify_host("198.20.0.1"), HostClass::Public);
        assert_eq!(classify_host("8.8.8.8"), HostClass::Public);
        assert_eq!(classify_host("::1"), HostClass::Loopback);
        assert_eq!(classify_host("[::1]"), HostClass::Loopback);
        assert_eq!(classify_host("fe80::1"), HostClass::LinkLocal);
        assert_eq!(classify_host("fd12:3456::1"), HostClass::Private);
        assert_eq!(classify_host("2001:db8::1"), HostClass::Private);
        assert_eq!(classify_host("100::1"), HostClass::Private);
        assert_eq!(classify_host("ff02::1"), HostClass::Private);
        assert_eq!(classify_host("fec0::1"), HostClass::Private);
        assert_eq!(classify_host("2001:2::1"), HostClass::Private);
        assert_eq!(classify_host("64:ff9b::a9fe:a9fe"), HostClass::LinkLocal);
        assert_eq!(classify_host("[64:ff9b::7f00:1]"), HostClass::Loopback);
        assert_eq!(classify_host("64:ff9b::808:808"), HostClass::Public);
        assert_eq!(classify_host("64:ff9b:1:a00:0:100::"), HostClass::Private);
        assert_eq!(classify_host("::7f00:1"), HostClass::Loopback);
        assert_eq!(classify_host("::a00:1"), HostClass::Private);
        assert_eq!(classify_host("::808:808"), HostClass::Public);
        assert_eq!(classify_host("::ffff:10.1.2.3"), HostClass::Private);
        assert_eq!(classify_host("2002:a9fe:a9fe::"), HostClass::LinkLocal);
        assert_eq!(classify_host("2002:808:808::"), HostClass::Public);
        assert_eq!(classify_host("2606:4700:4700::1111"), HostClass::Public);
        assert_eq!(classify_host("example.com"), HostClass::Name);
    }

    #[test]
    fn blocks_metadata_bounce() {
        assert!(private_redirect("example.com", "169.254.169.254"));
        assert!(private_redirect("example.com", "127.0.0.1"));
        assert!(private_redirect("example.com", "::1"));
        assert!(private_redirect("example.com", "0.0.0.1"));
        assert!(private_redirect("example.com", "224.0.0.1"));
        assert!(private_redirect("example.com", "ff02::1"));
        assert!(private_redirect("example.com", "64:ff9b::a9fe:a9fe"));
        assert!(private_redirect("example.com", "::7f00:1"));
        assert!(private_redirect("example.com", "2002:a00:1::"));
        assert!(!private_redirect("192.168.1.1", "192.168.1.1"));
        assert!(!private_redirect("example.com", "example.net"));
    }
}
