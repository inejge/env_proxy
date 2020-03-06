//! Determine proxy parameters for a URL from the environment.
//
// Copyright (c) 2016 Ivan Nejgebauer <inejge@gmail.com>
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.
//!
//! Environment variables are one way to request the use of an HTTP proxy server for
//! outgoing connections in many command-line applications. _Which_ environment variable
//! will be used depends on the target URL and the convention used by the application
//! (or, customarily, the connection library that it uses.)
//!
//! This crate aims to replicate the convention of the __curl__ library and offer it
//! behind a simple API: in most cases, a single function, `for_url()`, which accepts
//! a target URL and returns the proxy parameters, if applicable. The method for determining
//! the parameters is described in detail in that function's documentation.
//!
//! # Getting Started
//!
//! Add the following to the `[dependencies]` section of your `Cargo.toml`:
//!
//! ```toml
//! env_proxy = "0.3"
//! ```
//!
//! If you're using the 2015 edition of Rust, import the crate to your crate root:
//!
//! ```
//! extern crate env_proxy;
//! ```
//!
//! # Examples
//!
//! To determine proxy parameters for `http://www.example.org`:
//!
//! ```
//! # extern crate url;
//! # extern crate env_proxy;
//! # fn main() {
//! use env_proxy;
//! use url::Url;
//!
//! let url = Url::parse("http://www.example.org").unwrap();
//! if let Some(proxy) = env_proxy::for_url(&url).host_port() {
//!     println!("Proxy host: {}", proxy.0);
//!     println!("Proxy port: {}", proxy.1);
//! }
//! # }
//! ```


#[cfg(test)]
use lazy_static::lazy_static;
use log::warn;

use std::env::var_os;
use url::{self, Url};

macro_rules! env_var_pair {
    ($lc_var:expr, $uc_var:expr) => {
        var_os($lc_var).or_else(|| var_os($uc_var))
            .map(|v| v.to_str()
                .map(str::to_string)
                .or_else(|| {
                    warn!("non UTF-8 content in {}/{}", $lc_var, $uc_var);
                    None
                }))
            .unwrap_or_else(|| None)
    };
}

fn matches_no_proxy(url: &Url) -> bool {
    if let Some(no_proxy) = env_var_pair!("no_proxy", "NO_PROXY") {
        if no_proxy == "*" {
            return true;
        }
        if let Some(host) = url.host_str() {
            'elems: for elem in no_proxy.split(|c| c == ',' || c == ' ') {
                if elem == "" || elem == "." {
                    continue;
                }
                let ch1 = elem.chars().next().unwrap();
                let mut elem_iter = elem.chars();
                if ch1 == '.' {
                    elem_iter.next();
                }
                let mut elem_iter = elem_iter.rev();
                let mut host_iter = host.chars().rev();
                while let Some(elem_ch) = elem_iter.next() {
                    if let Some(host_ch) = host_iter.next() {
                        let host_ch = host_ch as u32;
                        let elem_ch = match elem_ch as u32 {
                            uppercase @ 0x41 ..= 0x5a => uppercase + 0x20,
                            anything => anything
                        };
                        if elem_ch == host_ch {
                            continue;
                        }
                        continue 'elems;
                    } else {
                        continue 'elems;
                    }
                }
                match host_iter.next() {
                    None => return true,
                    Some(host_ch) if host_ch == '.' => return true,
                    _ => ()
                }
            }
        }
    }
    false
}

/// A wrapper for the proxy URL retrieved from the environment.
///
/// This struct will wrap the raw value of the URL, which is only guaranteed to be valid UTF-8
/// when returned. Various methods exist to extract the value as-is, translate it into other forms,
/// and provide elements of interest.
pub struct ProxyUrl(Option<String>, Option<u16>);

impl ProxyUrl {
    /// Return the raw value of the proxy URL.
    pub fn raw_value(self) -> Option<String> {
        self.0
    }

    /// Return `true` if the `None` value is wrapped.
    pub fn is_none(self) -> bool {
        self.0.is_none()
    }

    /// Set the default port to use when transforming the raw URL value if
    /// the port isn't specified in the URL.
    ///
    /// A `ProxyUrl` instance returned by the library will have the default
    /// port set to __8080__, which corresponds to __http-alt__ in the IANA port
    /// registry. This is different from __curl__, which uses port 1080 as the default.
    ///
    /// To skip the default port substitution, use [`with_no_default_port()`]
    /// (#method.with_no_default_port) on the instance.
    pub fn with_default_port(self, port: u16) -> Self {
        ProxyUrl(self.0, Some(port))
    }

    /// Don't use the default port value when transforming the raw URL.
    pub fn with_no_default_port(self) -> Self {
        ProxyUrl(self.0, None)
    }

    /// Transform the raw proxy URL into a `Url`.
    ///
    /// The transformation will:
    ///
    /// * Parse the raw URL as a `Url`. If the raw URL lacks the scheme, `http` is assumed and
    ///   "http://" is prepended to canonicalize the value;
    /// * Ensure that the host part is not empty;
    /// * Use the default value for the port (or not, see [`with_default_port()`](#method.with_default_port))
    ///   if one is not specified in the raw URL.
    /// * Ensure that the port is not empty.
    ///
    /// If any of the steps fail, `None` will be returned.
    pub fn to_url(self) -> Option<Url> {
        let mut orig_scheme = self.0.as_ref().map(|s|
            if s.starts_with("http://") {
                Some("http")
            } else if s.starts_with("https://") {
                Some("https")
            } else {
                None
            }
        ).unwrap_or(None);
        if let Some(Ok(mut url)) = self.0.map(|mut s| {
            if !s.contains("://") {
                s.insert_str(0, "http://");
                orig_scheme = Some("http");
            }
            if orig_scheme.is_some() {
                s = s.replacen("http", "xttp", 1);
            }
            Url::parse(&s).map_err(|e| {
                warn!("url parse error: {}", e);
                e
            })
        }) {
            if url.host_str().is_none() {
                warn!("host part of the URL is empty");
                return None;
            }
            if let Some(orig_scheme) = orig_scheme {
                let port = url.port();
                url = match format!("{}{}", orig_scheme, &url[url::Position::AfterScheme..]).parse() {
                    Ok(url) => url,
                    Err(e) => {
                        warn!("could not set URL scheme back to {}: {}", orig_scheme, e);
                        return None;
                    },
                };
                if port.is_some() {
                    url.set_port(port).unwrap_or(());
                    return Some(url);
                }
            }
            if url.port().is_some() {
                return Some(url);
            } else if self.1.is_none() {
                warn!("the port of the URL is unknown");
                return None;
            }
            match url.set_port(self.1) {
                Ok(_) => return Some(url),
                Err(_) => warn!("could not set URL port"),
            }
        }
        None
    }

    /// Return the __(host, port)__ tuple of the proxy.
    ///
    /// The raw URL will first be transformed into a `Url`, with any errors in the conversion
    /// producing a `None` (see [`to_url()`](#method.to_url)).
    pub fn host_port(self) -> Option<(String, u16)> {
        self.to_url().map(|u| (u.host_str().expect("host_str").to_string(), u.port_or_known_default().expect("port")))
    }


    /// Return the string representation of the proxy URL.
    ///
    /// The raw URL will first be transformed into a `Url`, with any errors in the conversion
    /// producing a `None` (see [`to_url()`](#method.to_url)).
    pub fn to_string(self) -> Option<String> {
        self.to_url().map(Url::into_string)
    }
}

/// Determine proxy parameters for a URL by examining the environment variables.
///
/// __Attention__: in a multithreaded program, care should be taken not to change the environment
/// in multiple threads simultaneously without some form of serialization.
///
/// Most environment variables described here can be defined either with an all-lowercase or an
/// all-uppercase name. If both versions are defined, the all-lowercase name takes precedence
/// (e.g., __no_proxy__ will be used even if __NO_PROXY__ is defined.) The only exception is
/// __http_proxy__, where only the lowercase name is checked for. This text will use the
/// lowercase variants for simplicity.
///
/// If __no_proxy__ is defined, check the host part of the URL against its components and return
/// `None` if there is any match. The value of __no_proxy__ should be a space- or comma-separated
/// list of host/domain names or IP addresses for which no proxying should be done, or a single
/// '&#8239;__*__&#8239;' (asterisk) which means that proxying is disabled for all hosts. Empty names
/// are skipped. Names beginning with a dot are not treated specially; matching is always done
/// by full domain name component. A name consisting of a bare dot is skipped (this is different
/// from __curl__'s behavior.)
///
/// The rules are summarized in the following table, where the two rightmost columns are headed by two
/// __no_proxy__ components, the leftmost column contains hostnames, and the symbol inside each cell
/// represents the result of checking the hostname against the corresponding component:
///
/// |             |example.org|.example.org|
/// |-------------|:---------:|:----------:|
/// |example.org  |  &#x2714; |  &#x2714;  |
/// |a.example.org|  &#x2714; |  &#x2714;  |
/// |xample.org   |  &#x2718; |  &#x2718;  |
/// |org          |  &#x2718; |  &#x2718;  |
///
/// For the __ftp__ protocol scheme, __ftp_proxy__ is checked first; for __http__, __http_proxy__
/// is checked, and for __https__, it's __https_proxy__. These three schemes will fall back to __all_proxy__
/// if the original variable is undefined. For all other schemes only __all_proxy__ is checked. In this
/// context, "checked" means that the value of a variable is used if present, and the search for further
/// definitions stops.
///
/// The return value, if not `None`, is an opaque structure wrapping the value (possibly canonicalized,
/// see [`ProxyUrl::to_url()`](struct.ProxyUrl.html#method.to_url)) of the chosen environment variable.
///
/// If the target URL matches __no_proxy__, or if the hostname cannot be extracted from the URL,
/// the function returns `None`. If the port is not explicitly defined in the proxy URL, the value 8080
/// is used.
pub fn for_url(url: &Url) -> ProxyUrl {
    if matches_no_proxy(url) {
        return ProxyUrl(None, None);
    }

    let maybe_https_proxy = env_var_pair!("https_proxy", "HTTPS_PROXY");
    let maybe_ftp_proxy = env_var_pair!("ftp_proxy", "FTP_PROXY");
    let maybe_http_proxy = env_var_pair!("http_proxy", "");             // ugh, but it works
    let maybe_all_proxy = env_var_pair!("all_proxy", "ALL_PROXY");

    let url_value = match url.scheme() {
        "https" => maybe_https_proxy.or(maybe_all_proxy),
        "http" => maybe_http_proxy.or(maybe_all_proxy),
        "ftp" => maybe_ftp_proxy.or(maybe_all_proxy),
        _ => maybe_all_proxy,
    };
    ProxyUrl(url_value, Some(8080))
}

/// Determine proxy parameters for a URL given as a string.
///
/// Convert the given string to a URL and pass it to [`for_url()`](#method.for_url), returning
/// its result. If the conversion of the input argument fails, return `None`.
pub fn for_url_str<S: AsRef<str>>(s: S) -> ProxyUrl {
    let url = match Url::parse(s.as_ref()) {
        Ok(url) => url,
        Err(e) => {
            warn!("error parsing '{}' as Url: {}", s.as_ref(), e);
            return ProxyUrl(None, None);
        },
    };
    for_url(&url)
}

#[cfg(test)]
mod tests {
    use std::env::{remove_var, set_var};
    use std::sync::Mutex;
    use super::*;
    use url::Url;

    // environment is per-process, and we need it stable per-thread,
    // hence locking
    lazy_static! {
        static ref LOCK: Mutex<()> = Mutex::new(());
    }

    fn scrub_env() {
        remove_var("http_proxy");
        remove_var("https_proxy");
        remove_var("HTTPS_PROXY");
        remove_var("ftp_proxy");
        remove_var("FTP_PROXY");
        remove_var("all_proxy");
        remove_var("ALL_PROXY");
        remove_var("no_proxy");
        remove_var("NO_PROXY");
    }

    #[test]
    fn no_proxy_simple_name() {
        let _l = LOCK.lock();
        scrub_env();
        set_var("no_proxy", "example.org");
        set_var("http_proxy", "http://proxy.example.com:8080");
        let u = Url::parse("http://example.org").ok().unwrap();
        assert!(for_url(&u).is_none());
        assert!(for_url_str("http://example.org").is_none());
    }

    #[test]
    fn no_proxy_global() {
        let _l = LOCK.lock();
        scrub_env();
        set_var("no_proxy", "*");
        set_var("http_proxy", "http://proxy.example.com:8080");
        let u = Url::parse("http://example.org").ok().unwrap();
        assert!(for_url(&u).is_none());
        assert!(for_url_str("http://example.org").is_none());
    }

    #[test]
    fn no_proxy_subdomain() {
        let _l = LOCK.lock();
        scrub_env();
        set_var("no_proxy", "example.org");
        set_var("http_proxy", "http://proxy.example.com:8080");
        let u = Url::parse("http://www.example.org").ok().unwrap();
        assert!(for_url(&u).is_none());
    }

    #[test]
    fn no_proxy_subdomain_dot() {
        let _l = LOCK.lock();
        scrub_env();
        set_var("no_proxy", ".example.org");
        set_var("http_proxy", "http://proxy.example.com:8080");
        let u = Url::parse("http://www.example.org").ok().unwrap();
        assert!(for_url(&u).is_none());
        assert!(for_url_str("http://www.example.org").is_none());
    }

    #[test]
    fn no_proxy_multiple_list() {
        let _l = LOCK.lock();
        scrub_env();
        set_var("no_proxy", "com, .example.org, net");
        set_var("http_proxy", "http://proxy.example.com:8080");
        let u = Url::parse("http://www.example.org").ok().unwrap();
        assert!(for_url(&u).is_none());
        assert!(for_url_str("http://www.example.org").is_none());
    }

    #[test]
    fn http_proxy_specific() {
        let _l = LOCK.lock();
        scrub_env();
        set_var("http_proxy", "http://proxy.example.com:8080");
        set_var("all_proxy", "http://proxy.example.org:8081");
        let u = Url::parse("http://www.example.org").ok().unwrap();
        assert_eq!(for_url(&u).host_port(), Some(("proxy.example.com".to_string(), 8080)));
        assert_eq!(
            for_url_str("http://www.example.org").to_string(),
            Some("http://proxy.example.com:8080/".to_string())
        );
    }

    #[test]
    fn default_proxy_url_scheme() {
        let _l = LOCK.lock();
        scrub_env();
        set_var("http_proxy", "proxy.example.com:8080");
        let u = Url::parse("http://www.example.org").ok().unwrap();
        assert_eq!(for_url(&u).host_port(), Some(("proxy.example.com".to_string(), 8080)));
        assert_eq!(
            for_url_str("http://www.example.org").to_string(),
            Some("http://proxy.example.com:8080/".to_string())
        );
    }

    #[test]
    fn proxy_url_with_explicit_scheme_port() {
        let _l = LOCK.lock();
        scrub_env();
        set_var("http_proxy", "http://proxy.example.com:80");
        let u = Url::parse("http://www.example.org").ok().unwrap();
        assert_eq!(for_url(&u).host_port(), Some(("proxy.example.com".to_string(), 80)));
        assert_eq!(
            for_url_str("http://www.example.org").to_string(),
            Some("http://proxy.example.com/".to_string())
        );
        set_var("http_proxy", "https://proxy.example.com:443");
        assert_eq!(for_url(&u).host_port(), Some(("proxy.example.com".to_string(), 443)));
        assert_eq!(
            for_url_str("http://www.example.org").to_string(),
            Some("https://proxy.example.com/".to_string())
        );
    }

    #[test]
    fn proxy_url_without_port() {
        let _l = LOCK.lock();
        scrub_env();
        set_var("http_proxy", "http://proxy.example.com");
        let u = Url::parse("http://www.example.org").ok().unwrap();
        assert_eq!(for_url(&u).host_port(), Some(("proxy.example.com".to_string(), 8080)));
    }

    #[test]
    fn http_proxy_fallback() {
        let _l = LOCK.lock();
        scrub_env();
        set_var("ALL_PROXY", "http://proxy.example.com:8080");
        let u = Url::parse("http://www.example.org").ok().unwrap();
        assert_eq!(for_url(&u).host_port(), Some(("proxy.example.com".to_string(), 8080)));
        assert_eq!(
            for_url_str("http://www.example.org").to_string(),
            Some("http://proxy.example.com:8080/".to_string())
        );
        set_var("all_proxy", "http://proxy.example.org:8081");
        assert_eq!(for_url(&u).host_port(), Some(("proxy.example.org".to_string(), 8081)));
        assert_eq!(
            for_url_str("http://www.example.org").to_string(),
            Some("http://proxy.example.org:8081/".to_string())
        );
    }

    #[test]
    fn https_proxy_specific() {
        let _l = LOCK.lock();
        scrub_env();
        set_var("HTTPS_PROXY", "http://proxy.example.com:8080");
        set_var("all_proxy", "http://proxy.example.org:8081");
        let u = Url::parse("https://www.example.org").ok().unwrap();
        assert_eq!(for_url(&u).host_port(), Some(("proxy.example.com".to_string(), 8080)));
        assert_eq!(
            for_url_str("https://www.example.org").to_string(),
            Some("http://proxy.example.com:8080/".to_string())
        );
        set_var("https_proxy", "http://proxy.example.com:8082");
        assert_eq!(for_url(&u).host_port(), Some(("proxy.example.com".to_string(), 8082)));
        assert_eq!(
            for_url_str("https://www.example.org").to_string(),
            Some("http://proxy.example.com:8082/".to_string())
        );
    }

    #[test]
    fn https_proxy_fallback() {
        let _l = LOCK.lock();
        scrub_env();
        set_var("ALL_PROXY", "http://proxy.example.org:8081");
        let u = Url::parse("ftp://www.example.org").ok().unwrap();
        assert_eq!(for_url(&u).host_port(), Some(("proxy.example.org".to_string(), 8081)));
        assert_eq!(
            for_url_str("https://www.example.org").to_string(),
            Some("http://proxy.example.org:8081/".to_string())
        );
        set_var("all_proxy", "http://proxy.example.org:8082");
        assert_eq!(for_url(&u).host_port(), Some(("proxy.example.org".to_string(), 8082)));
        assert_eq!(
            for_url_str("https://www.example.org").to_string(),
            Some("http://proxy.example.org:8082/".to_string())
        );
    }

    #[test]
    fn ftp_proxy_specific() {
        let _l = LOCK.lock();
        scrub_env();
        set_var("FTP_PROXY", "http://proxy.example.com:8080");
        set_var("all_proxy", "http://proxy.example.org:8081");
        let u = Url::parse("ftp://www.example.org").ok().unwrap();
        assert_eq!(for_url(&u).host_port(), Some(("proxy.example.com".to_string(), 8080)));
        assert_eq!(
            for_url_str("ftp://www.example.org").to_string(),
            Some("http://proxy.example.com:8080/".to_string())
        );
        set_var("ftp_proxy", "http://proxy.example.com:8082");
        assert_eq!(for_url(&u).host_port(), Some(("proxy.example.com".to_string(), 8082)));
        assert_eq!(
            for_url_str("ftp://www.example.org").to_string(),
            Some("http://proxy.example.com:8082/".to_string())
        );
    }

    #[test]
    fn ftp_proxy_fallback() {
        let _l = LOCK.lock();
        scrub_env();
        set_var("ALL_PROXY", "http://proxy.example.org:8081");
        let u = Url::parse("ftp://www.example.org").ok().unwrap();
        assert_eq!(for_url(&u).host_port(), Some(("proxy.example.org".to_string(), 8081)));
        assert_eq!(
            for_url_str("ftp://www.example.org").to_string(),
            Some("http://proxy.example.org:8081/".to_string())
        );
        set_var("all_proxy", "http://proxy.example.org:8082");
        assert_eq!(for_url(&u).host_port(), Some(("proxy.example.org".to_string(), 8082)));
        assert_eq!(
            for_url_str("ftp://www.example.org").to_string(),
            Some("http://proxy.example.org:8082/".to_string())
        );
    }
}
