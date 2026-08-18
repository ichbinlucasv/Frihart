//! Identity autofill. Passwords are never stored.

#![forbid(unsafe_code)]

use std::path::Path;

use serde::{Deserialize, Serialize};

use frihart_core::{Result, write_private_str};
use frihart_html::FormField;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FieldKind {
    Name,
    Email,
    Organization,
    Address,
    City,
    Postal,
    Country,
    Phone,
    Username,
    Password,
    Other,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Identity {
    pub name: String,
    pub email: String,
    pub organization: String,
    pub address: String,
    pub city: String,
    pub postal: String,
    pub country: String,
    pub phone: String,
}

impl Identity {
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = std::fs::read_to_string(path)?;
        Ok(toml::from_str(&text).unwrap_or_default())
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let text = toml::to_string_pretty(self).unwrap_or_default();
        write_private_str(path, &text)
    }

    pub fn is_empty(&self) -> bool {
        self.name.is_empty()
            && self.email.is_empty()
            && self.organization.is_empty()
            && self.address.is_empty()
            && self.city.is_empty()
            && self.postal.is_empty()
            && self.country.is_empty()
            && self.phone.is_empty()
    }

    pub fn value_for(&self, kind: FieldKind) -> Option<&str> {
        let v = match kind {
            FieldKind::Name | FieldKind::Username => self.name.as_str(),
            FieldKind::Email => self.email.as_str(),
            FieldKind::Organization => self.organization.as_str(),
            FieldKind::Address => self.address.as_str(),
            FieldKind::City => self.city.as_str(),
            FieldKind::Postal => self.postal.as_str(),
            FieldKind::Country => self.country.as_str(),
            FieldKind::Phone => self.phone.as_str(),
            FieldKind::Password | FieldKind::Other => return None,
        };
        if v.is_empty() { None } else { Some(v) }
    }
}

pub fn classify(field: &FormField) -> FieldKind {
    let t = field.input_type.to_ascii_lowercase();
    if t == "password" {
        return FieldKind::Password;
    }
    if t == "email" {
        return FieldKind::Email;
    }
    if t == "tel" {
        return FieldKind::Phone;
    }
    let blob =
        format!("{} {} {}", field.autocomplete, field.name, field.label).to_ascii_lowercase();
    if blob.contains("password") || blob.contains("passwd") {
        return FieldKind::Password;
    }
    if blob.contains("email") || blob.contains("e-mail") {
        return FieldKind::Email;
    }
    if blob.contains("phone") || blob.contains("tel") || blob.contains("mobile") {
        return FieldKind::Phone;
    }
    if blob.contains("organiz") || blob.contains("company") {
        return FieldKind::Organization;
    }
    if blob.contains("address") || blob.contains("street") {
        return FieldKind::Address;
    }
    if blob.contains("city") || blob.contains("town") {
        return FieldKind::City;
    }
    if blob.contains("zip") || blob.contains("postal") {
        return FieldKind::Postal;
    }
    if blob.contains("country") {
        return FieldKind::Country;
    }
    if blob.contains("user") || blob.contains("login") {
        return FieldKind::Username;
    }
    if blob.contains("name") {
        return FieldKind::Name;
    }
    FieldKind::Other
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn never_fills_password() {
        let id = Identity {
            name: "Ada".into(),
            email: "a@b.c".into(),
            ..Identity::default()
        };
        assert!(id.value_for(FieldKind::Password).is_none());
        assert_eq!(id.value_for(FieldKind::Email), Some("a@b.c"));
    }

    #[test]
    fn classifies_password_and_email() {
        let p = FormField {
            name: "pass".into(),
            id: String::new(),
            input_type: "password".into(),
            autocomplete: String::new(),
            label: String::new(),
        };
        assert_eq!(classify(&p), FieldKind::Password);
        let e = FormField {
            name: "mail".into(),
            id: String::new(),
            input_type: "text".into(),
            autocomplete: "email".into(),
            label: String::new(),
        };
        assert_eq!(classify(&e), FieldKind::Email);
    }
}
