//! Pertahanan SSRF berlapis:
//! 1. Validasi URL (skema http/https, tanpa userinfo, host tidak boleh loopback/privat).
//! 2. Resolver DNS khusus (`SafeResolver`) yang membuang alamat privat saat koneksi,
//!    sehingga DNS rebinding dan IP privat dari hasil resolve tetap terblokir.
//! 3. Redirect dimatikan di klien HTTP agar tidak bisa dialihkan ke alamat internal.

use reqwest::dns::{Addrs, Name, Resolve, Resolving};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::Arc;
use url::{Host, Url};

/// True jika alamat termasuk alamat publik yang boleh dituju.
pub fn is_public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => is_public_v4(v4),
        IpAddr::V6(v6) => {
            if let Some(v4) = v6.to_ipv4_mapped() {
                return is_public_v4(v4);
            }
            is_public_v6(v6)
        }
    }
}

fn is_public_v4(ip: Ipv4Addr) -> bool {
    let o = ip.octets();
    !(ip.is_loopback()
        || ip.is_private()
        || ip.is_link_local()
        || ip.is_unspecified()
        || ip.is_broadcast()
        || ip.is_multicast()
        || ip.is_documentation()
        // Shared address space 100.64.0.0/10 (CGNAT)
        || (o[0] == 100 && (o[1] & 0xC0) == 64)
        // 192.0.0.0/24 (IETF protocol assignments)
        || (o[0] == 192 && o[1] == 0 && o[2] == 0)
        // 198.18.0.0/15 (benchmarking)
        || (o[0] == 198 && (o[1] & 0xFE) == 18)
        || o[0] >= 240)
}

fn is_public_v6(ip: Ipv6Addr) -> bool {
    let s = ip.segments();
    !(ip.is_loopback()
        || ip.is_unspecified()
        || ip.is_multicast()
        // Unique local fc00::/7
        || (s[0] & 0xFE00) == 0xFC00
        // Link-local fe80::/10
        || (s[0] & 0xFFC0) == 0xFE80
        // Documentation 2001:db8::/32
        || (s[0] == 0x2001 && s[1] == 0x0DB8))
}

/// Mencocokkan host persis (case-insensitive) dengan daftar whitelist.
pub fn host_allowed(host: &str, list: &[String]) -> bool {
    list.iter().any(|h| h.eq_ignore_ascii_case(host))
}

/// Validasi URL target sebelum dikirim.
/// `bypass_hosts = None`  -> mode normal: host harus publik.
/// `bypass_hosts = Some(list)` -> host wajib ada di `list`; host internal diizinkan.
pub fn check_target(raw: &str, bypass_hosts: Option<&[String]>) -> Result<Url, &'static str> {
    let url = Url::parse(raw).map_err(|_| "Invalid URL format")?;
    if url.scheme() != "http" && url.scheme() != "https" {
        return Err("Forbidden: only http and https are allowed");
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err("Forbidden: credentials in URL are not allowed");
    }
    let host = url.host().ok_or("No host found")?;

    match bypass_hosts {
        Some(list) => {
            if list.is_empty() {
                return Err("Bypass not configured");
            }
            let name = url.host_str().unwrap_or_default();
            if !host_allowed(name, list) {
                return Err("URL not in bypass whitelist");
            }
            Ok(url)
        }
        None => {
            match host {
                Host::Ipv4(ip) => {
                    if !is_public_ip(IpAddr::V4(ip)) {
                        return Err("Forbidden: Cannot proxy to internal addresses");
                    }
                }
                Host::Ipv6(ip) => {
                    if !is_public_ip(IpAddr::V6(ip)) {
                        return Err("Forbidden: Cannot proxy to internal addresses");
                    }
                }
                Host::Domain(name) => {
                    let lower = name.to_ascii_lowercase();
                    let blocked_name = lower == "localhost"
                        || lower.ends_with(".localhost")
                        || lower.ends_with(".local")
                        || lower.ends_with(".internal")
                        || lower.ends_with(".home.arpa");
                    if blocked_name {
                        return Err("Forbidden: Cannot proxy to internal addresses");
                    }
                }
            }
            Ok(url)
        }
    }
}

/// Resolver DNS yang hanya mengizinkan alamat publik, kecuali host yang ada
/// di `allow_private` (hanya dipakai klien mode bypass).
pub struct SafeResolver {
    allow_private: Arc<Vec<String>>,
}

impl SafeResolver {
    pub fn new(allow_private: Vec<String>) -> Self {
        Self {
            allow_private: Arc::new(allow_private),
        }
    }
}

impl Resolve for SafeResolver {
    fn resolve(&self, name: Name) -> Resolving {
        let host = name.as_str().to_string();
        let allow = self.allow_private.clone();
        Box::pin(async move {
            let found: Vec<SocketAddr> =
                tokio::net::lookup_host((host.as_str(), 0)).await?.collect();
            let private_ok = host_allowed(&host, &allow);
            let kept: Vec<SocketAddr> = found
                .into_iter()
                .filter(|a| private_ok || is_public_ip(a.ip()))
                .collect();
            if kept.is_empty() {
                return Err("Forbidden: target resolves only to non-public addresses".into());
            }
            let addrs: Addrs = Box::new(kept.into_iter());
            Ok(addrs)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_addresses() {
        assert!(!is_public_ip("127.0.0.1".parse().unwrap()));
        assert!(!is_public_ip("10.1.2.3".parse().unwrap()));
        assert!(!is_public_ip("172.20.0.5".parse().unwrap()));
        assert!(!is_public_ip("169.254.169.254".parse().unwrap()));
        assert!(!is_public_ip("::1".parse().unwrap()));
        assert!(!is_public_ip("::ffff:127.0.0.1".parse().unwrap()));
        assert!(!is_public_ip("fd00::1".parse().unwrap()));
        assert!(is_public_ip("8.8.8.8".parse().unwrap()));
        assert!(is_public_ip("2606:4700:4700::1111".parse().unwrap()));
    }

    #[test]
    fn strict_mode_blocks_internal_targets() {
        assert!(check_target("http://127.0.0.1:8080/", None).is_err());
        assert!(check_target("http://localhost/x", None).is_err());
        assert!(check_target("http://[::1]/x", None).is_err());
        assert!(check_target("file:///etc/passwd", None).is_err());
        assert!(check_target("http://user:pw@example.com/", None).is_err());
        assert!(check_target("https://example.com/api?x=1", None).is_ok());
    }

    #[test]
    fn bypass_requires_exact_whitelisted_host() {
        let list = vec!["backend.internal.example".to_string()];
        assert!(check_target("http://backend.internal.example/a", Some(&list)).is_ok());
        assert!(check_target("http://evil.com/?h=backend.internal.example", Some(&list)).is_err());
        assert!(check_target("http://x/a", Some(&[])).is_err());
    }
}
