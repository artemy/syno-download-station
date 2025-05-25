use std::fs;
use wiremock::{Match, Request};

/// # Panics
///
/// Will panic if a file can't be read or missing
#[must_use = "This function returns the body of the file as a string"]
pub fn body_from_file(path: &str) -> String {
    fs::read_to_string(path).expect("Failed to read file")
}

pub struct FormParamExactMatcher(String, String);

impl FormParamExactMatcher {
    /// Specify the expected value for a form parameter.
    pub fn new<K: Into<String>, V: Into<String>>(key: K, value: V) -> Self {
        let key = key.into();
        let value = value.into();
        Self(key, value)
    }
}

/// Shorthand for [`FormParamExactMatcher::new`].
pub fn form_param<K, V>(key: K, value: V) -> FormParamExactMatcher
where
    K: Into<String>,
    V: Into<String>,
{
    FormParamExactMatcher::new(key, value)
}

impl Match for FormParamExactMatcher {
    fn matches(&self, request: &Request) -> bool {
        form_urlencoded::parse(&request.body)
            .any(|q| q.0 == self.0.as_str() && q.1 == self.1.as_str())
    }
}
