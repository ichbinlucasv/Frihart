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
    if ip.is_private() || ip.is_documentation() || ip.is_broadcast() {
        return HostClass::Private;
    }
    let o = ip.octets();
    if o[0] == 100 && (o[1] & 0b1100_0000) == 64 {
        return HostClass::Private;
    }
    if o[0] == 198 && (o[1] == 18 || o[1] == 19) {
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
    if (s[0] & 0xfe00) == 0xfc00 {
        return HostClass::Private;
    }
    if s[0] == 0x2001 && s[1] == 0x0db8 {
        return HostClass::Private;
    }
    if let Some(v4) = ip.to_ipv4_mapped() {
        return classify_v4(v4);
    }
    HostClass::Public
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
        assert_eq!(classify_host("8.8.8.8"), HostClass::Public);
        assert_eq!(classify_host("::1"), HostClass::Loopback);
        assert_eq!(classify_host("[::1]"), HostClass::Loopback);
        assert_eq!(classify_host("fe80::1"), HostClass::LinkLocal);
        assert_eq!(classify_host("fd12:3456::1"), HostClass::Private);
        assert_eq!(classify_host("2001:db8::1"), HostClass::Private);
        assert_eq!(classify_host("2606:4700:4700::1111"), HostClass::Public);
        assert_eq!(classify_host("example.com"), HostClass::Name);
    }

    #[test]
    fn blocks_metadata_bounce() {
        assert!(private_redirect("example.com", "169.254.169.254"));
        assert!(private_redirect("example.com", "127.0.0.1"));
        assert!(private_redirect("example.com", "::1"));
        assert!(!private_redirect("192.168.1.1", "192.168.1.1"));
        assert!(!private_redirect("example.com", "example.net"));
    }
}
