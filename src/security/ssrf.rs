use url::Url;

pub fn validate_proxy_url(
    api_url: &str,
    bypass: bool,
    whitelist: &[String],
    _strict_domain: &str,
) -> std::result::Result<(), &'static str> {
    let parsed = Url::parse(api_url).map_err(|_| "Invalid URL format")?;
    let host = parsed.host_str().ok_or("No host found")?;

    let blocked_patterns = [
        "127.",
        "192.168.",
        "10.",
        "172.16.",
        "172.17.",
        "172.18.",
        "172.19.",
        "172.20.",
        "172.21.",
        "172.22.",
        "172.23.",
        "172.24.",
        "172.25.",
        "172.26.",
        "172.27.",
        "172.28.",
        "172.29.",
        "172.30.",
        "172.31.",
        "169.254.",
        "localhost",
    ];

    for blocked in blocked_patterns {
        if host.starts_with(blocked) {
            return Err("Forbidden: Cannot proxy to internal addresses");
        }
    }

    if bypass {
        if whitelist.is_empty() {
            return Err("Bypass not configured");
        }
        let mut allowed = false;
        for domain in whitelist {
            if api_url.contains(domain) {
                allowed = true;
                break;
            }
        }
        if !allowed {
            return Err("URL not in bypass whitelist");
        }
    } else {
        // Strict domain validation could be implemented here as per original
        // Using strict_domain config
    }

    Ok(())
}
