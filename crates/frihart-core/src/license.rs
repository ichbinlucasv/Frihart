//! Offline pricing. Linux is free. No license server.

/// EUR lifetime price. `None` means this OS is free.
pub fn list_price_eur() -> Option<u32> {
    match std::env::consts::OS {
        "linux" => None,
        "android" => Some(80),
        "windows" | "macos" => Some(100),
        _ => Some(100),
    }
}

pub fn price_label() -> &'static str {
    match list_price_eur() {
        None => "Linux: free",
        Some(80) => "Android: €80 lifetime",
        Some(100) => "Windows / macOS: €100 lifetime",
        Some(_) => "paid lifetime",
    }
}

pub fn linux_is_free() -> bool {
    cfg!(target_os = "linux")
}

/// Local key only. Never phones home. Linux always granted.
pub fn licensed_locally(key_present: bool) -> bool {
    linux_is_free() || key_present
}
