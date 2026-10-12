//! Offline pricing. Linux is free. No license server.

/// EUR lifetime price. `None` means this OS is free.
pub fn list_price_eur() -> Option<u32> {
    if std::env::consts::OS == "linux" {
        None
    } else {
        Some(149)
    }
}

pub fn price_label() -> &'static str {
    if list_price_eur().is_none() {
        "Linux: free"
    } else {
        "Other OS: €4.99/month, €49/year or €149 lifetime"
    }
}

pub fn linux_is_free() -> bool {
    cfg!(target_os = "linux")
}

/// Local key only. Never phones home. Linux always granted.
pub fn licensed_locally(key_present: bool) -> bool {
    linux_is_free() || key_present
}

#[cfg(test)]
mod tests {
    #[test]
    fn linux_has_no_price() {
        if cfg!(target_os = "linux") {
            assert_eq!(super::list_price_eur(), None);
        } else {
            assert_eq!(super::list_price_eur(), Some(149));
        }
    }
}
