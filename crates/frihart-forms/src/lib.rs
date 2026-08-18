//! GET/POST encoding. Secrets never leave the machine via GET.

#![forbid(unsafe_code)]

use url::Url;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Field {
    pub name: String,
    pub value: String,
    pub secret: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Submit {
    pub action: String,
    pub method: String,
    pub fields: Vec<Field>,
}

impl Submit {
    pub fn method_norm(&self) -> &str {
        if self.method.is_empty() {
            "get"
        } else {
            self.method.as_str()
        }
    }

    pub fn is_get(&self) -> bool {
        self.method_norm().eq_ignore_ascii_case("get")
    }

    pub fn is_post(&self) -> bool {
        self.method_norm().eq_ignore_ascii_case("post")
    }

    pub fn get_url(&self, base: &Url) -> Option<Url> {
        if !self.is_get() {
            return None;
        }
        let mut u = self.target(base)?;
        {
            let mut qp = u.query_pairs_mut();
            qp.clear();
            for (k, v) in self.pairs() {
                qp.append_pair(k, v);
            }
        }
        Some(u)
    }

    /// `application/x-www-form-urlencoded` body. Secrets stay out.
    pub fn post_body(&self) -> Option<String> {
        if !self.is_post() {
            return None;
        }
        let mut ser = url::form_urlencoded::Serializer::new(String::new());
        for (k, v) in self.pairs() {
            ser.append_pair(k, v);
        }
        Some(ser.finish())
    }

    pub fn target(&self, base: &Url) -> Option<Url> {
        if self.action.is_empty() {
            Some(base.clone())
        } else {
            base.join(&self.action).ok()
        }
    }

    fn pairs(&self) -> impl Iterator<Item = (&str, &str)> {
        self.fields.iter().filter_map(|f| {
            if f.secret || f.name.is_empty() {
                None
            } else {
                Some((f.name.as_str(), f.value.as_str()))
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(method: &str) -> Submit {
        Submit {
            action: "/s".into(),
            method: method.into(),
            fields: vec![
                Field {
                    name: "q".into(),
                    value: "hi".into(),
                    secret: false,
                },
                Field {
                    name: "p".into(),
                    value: "nope".into(),
                    secret: true,
                },
            ],
        }
    }

    #[test]
    fn get_skips_secrets() {
        let base = Url::parse("https://ex.test/form").unwrap();
        let u = sample("GET").get_url(&base).unwrap();
        assert!(u.as_str().contains("q=hi"));
        assert!(!u.as_str().contains("nope"));
    }

    #[test]
    fn post_encodes_without_secrets() {
        let body = sample("POST").post_body().unwrap();
        assert!(body.contains("q=hi"));
        assert!(!body.contains("nope"));
        assert!(sample("GET").post_body().is_none());
    }
}
