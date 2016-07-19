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
//! env_proxy = "0.1"
//! ```
//!
//! Also, import the crate to your crate root:
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
//! if let Some(proxy) = env_proxy::for_url(&url) {
//!     println!("Proxy host: {}", proxy.0);
//!     println!("Proxy port: {}", proxy.1);
//! }
//! # }
//! ```


#[cfg(test)]
#[macro_use] extern crate lazy_static;
extern crate url;

use std::env::var_os;
use url::Url;

fn matches_no_proxy(url: &Url) -> bool {
    let mut maybe_no_proxy = var_os("no_proxy").map(|ref v| v.to_str().unwrap_or("").to_string());
    if maybe_no_proxy.is_none() {
	maybe_no_proxy = var_os("NO_PROXY").map(|ref v| v.to_str().unwrap_or("").to_string());
    }
    if let Some(no_proxy) = maybe_no_proxy {
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
			    uppercase @ 0x41 ... 0x5a => uppercase + 0x20,
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
/// from __curl__'s behavior.) The rules are summarized in the following table:
///
/// |             |example.org|.example.org|
/// |-------------|:---------:|:----------:|
/// |example.org  |  &#x2714; |  &#x2714;  |
/// |a.example.org|  &#x2714; |  &#x2714;  |
/// |xample.org   |  &#x2718; |  &#x2718;  |
/// |org          |  &#x2718; |  &#x2718;  |
///
/// For the __ftp__ protocol scheme, __ftp_proxy__ is checked first; for __https__, __https_proxy__
/// is checked. Both schemes will fall back to __http_proxy__, then __all_proxy__ if the former is
/// undefined. For __http__, __http_proxy__ is cheked first, then __all_proxy__. For all other schemes
/// only __all_proxy__ is checked. In this context, "checked" means that the value of a variable is used
/// if present, and the search for further definition stops.
///
/// The return value, if not `None`, is a tuple consisting of the proxy hostname and the port, which
/// are obtained from the chosen environment variable parsed as a URL.
///
/// If the target URL matches __no_proxy__, or if the hostname cannot be extracted from the URL,
/// the function returns `None`. If the port is not explicitly defined in the proxy URL, the value 8080
/// is used, which corresponds to __http-alt__ in the IANA port registry. This is different from __curl__,
/// which uses port 1080 as the default.

pub fn for_url(url: &Url) -> Option<(String, u16)> {
    if matches_no_proxy(url) {
	return None;
    }

    let mut maybe_https_proxy = var_os("https_proxy").map(|ref v| v.to_str().unwrap_or("").to_string());
    if maybe_https_proxy.is_none() {
	maybe_https_proxy = var_os("HTTPS_PROXY").map(|ref v| v.to_str().unwrap_or("").to_string());
    }
    let mut maybe_ftp_proxy = var_os("ftp_proxy").map(|ref v| v.to_str().unwrap_or("").to_string());
    if maybe_ftp_proxy.is_none() {
	maybe_ftp_proxy = var_os("FTP_PROXY").map(|ref v| v.to_str().unwrap_or("").to_string());
    }
    let maybe_http_proxy = var_os("http_proxy").map(|ref v| v.to_str().unwrap_or("").to_string());
    let mut maybe_all_proxy = var_os("all_proxy").map(|ref v| v.to_str().unwrap_or("").to_string());
    if maybe_all_proxy.is_none() {
	maybe_all_proxy = var_os("ALL_PROXY").map(|ref v| v.to_str().unwrap_or("").to_string());
    }
    if let Some(url_value) = match url.scheme() {
				 "https" => maybe_https_proxy.or(maybe_http_proxy.or(maybe_all_proxy)),
				 "http" => maybe_http_proxy.or(maybe_all_proxy),
				 "ftp" => maybe_ftp_proxy.or(maybe_http_proxy.or(maybe_all_proxy)),
				 _ => maybe_all_proxy,
			     } {
	if let Ok(proxy_url) = Url::parse(&url_value) {
	    if let Some(host) = proxy_url.host_str() {
		let port = proxy_url.port().unwrap_or(8080);
		return Some((host.to_string(), port));
	    }
	}
    }
    None
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
    }

    #[test]
    fn no_proxy_global() {
	let _l = LOCK.lock();
	scrub_env();
	set_var("no_proxy", "*");
	set_var("http_proxy", "http://proxy.example.com:8080");
	let u = Url::parse("http://example.org").ok().unwrap();
	assert!(for_url(&u).is_none());
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
    }

    #[test]
    fn no_proxy_multiple_list() {
	let _l = LOCK.lock();
	scrub_env();
	set_var("no_proxy", "com, .example.org, net");
	set_var("http_proxy", "http://proxy.example.com:8080");
	let u = Url::parse("http://www.example.org").ok().unwrap();
	assert!(for_url(&u).is_none());
    }

    #[test]
    fn http_proxy_specific() {
	let _l = LOCK.lock();
	scrub_env();
	set_var("http_proxy", "http://proxy.example.com:8080");
	set_var("all_proxy", "http://proxy.example.org:8081");
	let u = Url::parse("http://www.example.org").ok().unwrap();
	assert_eq!(for_url(&u), Some(("proxy.example.com".to_string(), 8080)));
    }

    #[test]
    fn http_proxy_fallback() {
	let _l = LOCK.lock();
	scrub_env();
	set_var("ALL_PROXY", "http://proxy.example.com:8080");
	let u = Url::parse("http://www.example.org").ok().unwrap();
	assert_eq!(for_url(&u), Some(("proxy.example.com".to_string(), 8080)));
	set_var("all_proxy", "http://proxy.example.org:8081");
	assert_eq!(for_url(&u), Some(("proxy.example.org".to_string(), 8081)));
    }

    #[test]
    fn https_proxy_specific() {
	let _l = LOCK.lock();
	scrub_env();
	set_var("HTTPS_PROXY", "http://proxy.example.com:8080");
	set_var("http_proxy", "http://proxy.example.org:8081");
	set_var("all_proxy", "http://proxy.example.org:8081");
	let u = Url::parse("https://www.example.org").ok().unwrap();
	assert_eq!(for_url(&u), Some(("proxy.example.com".to_string(), 8080)));
	set_var("https_proxy", "http://proxy.example.com:8081");
	assert_eq!(for_url(&u), Some(("proxy.example.com".to_string(), 8081)));
    }

    #[test]
    fn https_proxy_fallback() {
	let _l = LOCK.lock();
	scrub_env();
	set_var("http_proxy", "http://proxy.example.com:8080");
	set_var("ALL_PROXY", "http://proxy.example.org:8081");
	let u = Url::parse("https://www.example.org").ok().unwrap();
	assert_eq!(for_url(&u), Some(("proxy.example.com".to_string(), 8080)));
	remove_var("http_proxy");
	assert_eq!(for_url(&u), Some(("proxy.example.org".to_string(), 8081)));
	set_var("all_proxy", "http://proxy.example.org:8082");
	assert_eq!(for_url(&u), Some(("proxy.example.org".to_string(), 8082)));
    }

    #[test]
    fn ftp_proxy_specific() {
	let _l = LOCK.lock();
	scrub_env();
	set_var("FTP_PROXY", "http://proxy.example.com:8080");
	set_var("http_proxy", "http://proxy.example.org:8081");
	set_var("all_proxy", "http://proxy.example.org:8081");
	let u = Url::parse("ftp://www.example.org").ok().unwrap();
	assert_eq!(for_url(&u), Some(("proxy.example.com".to_string(), 8080)));
	set_var("ftp_proxy", "http://proxy.example.com:8081");
	assert_eq!(for_url(&u), Some(("proxy.example.com".to_string(), 8081)));
    }

    #[test]
    fn ftp_proxy_fallback() {
	let _l = LOCK.lock();
	scrub_env();
	set_var("http_proxy", "http://proxy.example.com:8080");
	set_var("ALL_PROXY", "http://proxy.example.org:8081");
	let u = Url::parse("ftp://www.example.org").ok().unwrap();
	assert_eq!(for_url(&u), Some(("proxy.example.com".to_string(), 8080)));
	remove_var("http_proxy");
	assert_eq!(for_url(&u), Some(("proxy.example.org".to_string(), 8081)));
	set_var("all_proxy", "http://proxy.example.org:8082");
	assert_eq!(for_url(&u), Some(("proxy.example.org".to_string(), 8082)));
    }
}
