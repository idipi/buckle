//! High-performance environment variable sanitization.
//! Strips sensitive credentials, tokens, and secrets before spawning untrusted processes.

use std::collections::{HashMap, HashSet};
use std::ffi::CString;

const SENSITIVE_PATTERNS: &[&str] = &[
    "TOKEN", "SECRET", "PASSWORD", "PASSWD",
    "AUTH", "CREDENTIAL", "PRIVATE", "API_KEY"
];

const SAFE_EXCEPTIONS: &[&str] = &[
    "TERM", "COLORTERM", "LANG", "LC_ALL", "LC_CTYPE",
    "BUCKLE_ACTIVE", "BUCKLE_WORKSPACE",
    "AGENT_SANDBOX_ACTIVE", "AGENT_SANDBOX_WORKSPACE",
    "EDITOR", "PAGER", "PATH", "HOME",
    "USER", "LOGNAME", "SHELL", "PWD", "SHLVL"
];

pub fn sanitize_environment(
    pass_vars: &[String],
    set_vars: &HashMap<String, String>,
) -> HashMap<String, String> {
    let mut clean = HashMap::new();
    let pass_set: HashSet<&str> = pass_vars.iter().map(|s| s.as_str()).collect();
    let safe_set: HashSet<&str> = SAFE_EXCEPTIONS.iter().cloned().collect();

    for (k, v) in std::env::vars() {
        if pass_set.contains(k.as_str()) || safe_set.contains(k.as_str()) {
            clean.insert(k, v);
            continue;
        }

        let k_upper = k.to_uppercase();
        let is_sensitive = SENSITIVE_PATTERNS.iter().any(|&pat| k_upper.contains(pat));
        if !is_sensitive {
            clean.insert(k, v);
        }
    }

    // Apply custom overrides
    for (k, v) in set_vars {
        clean.insert(k.clone(), v.clone());
    }

    clean
}

/// Convert environment map into a NULL-terminated array of C strings for execve.
pub fn to_c_env(env_map: &HashMap<String, String>) -> (Vec<CString>, Vec<*const libc::c_char>) {
    let mut c_strings = Vec::with_capacity(env_map.len());
    let mut c_ptrs = Vec::with_capacity(env_map.len() + 1);

    for (k, v) in env_map {
        if let Ok(cs) = CString::new(format!("{}={}", k, v)) {
            c_ptrs.push(cs.as_ptr());
            c_strings.push(cs);
        }
    }
    c_ptrs.push(std::ptr::null()); // NULL terminator

    (c_strings, c_ptrs)
}
