//! Native POSIX process management and execution pipeline.
//! Handles fork(), in-memory sandbox_init(), TTY pgrp, resource limits, and RAII cleanup.

use anyhow::{bail, Context, Result};
use std::collections::HashMap;
use std::ffi::CString;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicI32, Ordering};

use crate::env::{sanitize_environment, to_c_env};
use crate::ffi::apply_sandbox_profile;

static ACTIVE_CHILD_PGID: AtomicI32 = AtomicI32::new(0);

/// RAII Guard managing an ephemeral session directory in /private/tmp.
/// Automatically purges all temporary compiler artifacts and session data on drop.
pub struct TempSessionGuard {
    pub session_dir: PathBuf,
}

impl TempSessionGuard {
    pub fn new() -> Result<Self> {
        let pid = unsafe { libc::getpid() };
        let rand_suffix: u32 = rand_simple();
        let session_dir = PathBuf::from(format!("/private/tmp/agent_sandbox/session_{}_{:06x}", pid, rand_suffix));
        std::fs::create_dir_all(&session_dir)
            .with_context(|| format!("Failed to create session dir: {:?}", session_dir))?;
        Ok(Self { session_dir })
    }
}

impl Drop for TempSessionGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.session_dir);
    }
}

fn rand_simple() -> u32 {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    (now.as_nanos() & 0xFFFFFF) as u32
}

/// Locate an executable binary in PATH if it doesn't contain a path separator.
pub fn resolve_binary(bin_name: &str) -> Result<PathBuf> {
    if bin_name.contains('/') {
        let p = PathBuf::from(bin_name);
        if p.exists() {
            return Ok(p.canonicalize().unwrap_or(p));
        }
        return Ok(p);
    }

    if let Ok(path_var) = std::env::var("PATH") {
        for dir in path_var.split(':') {
            let candidate = Path::new(dir).join(bin_name);
            if candidate.is_file() {
                // Check execute permission
                if unsafe { libc::access(CString::new(candidate.to_string_lossy().as_bytes())?.as_ptr(), libc::X_OK) } == 0 {
                    return Ok(candidate);
                }
            }
        }
    }

    Ok(PathBuf::from(bin_name))
}

pub struct RunOptions<'a> {
    pub command: &'a [String],
    pub workspace: &'a Path,
    pub profile_content: &'a str,
    pub pass_env: &'a [String],
    pub custom_env: &'a HashMap<String, String>,
    pub is_interactive_shell: bool,
}

pub fn execute_sandboxed_process(options: &RunOptions) -> Result<i32> {
    if options.command.is_empty() {
        bail!("No command specified for execution");
    }

    let raw_bin = &options.command[0];
    let resolved_bin = resolve_binary(raw_bin)?;
    let bin_c = CString::new(resolved_bin.to_string_lossy().as_bytes())?;

    // Prepare argument array for execve
    let mut c_args = Vec::with_capacity(options.command.len());
    let mut c_arg_ptrs = Vec::with_capacity(options.command.len() + 1);
    for arg in options.command {
        let ca = CString::new(arg.as_str())?;
        c_arg_ptrs.push(ca.as_ptr());
        c_args.push(ca);
    }
    c_arg_ptrs.push(std::ptr::null());

    // 1. Initialize Ephemeral Session Storage (RAII)
    let temp_guard = TempSessionGuard::new()?;

    // 2. Prepare Environment Variables
    let mut env_map = sanitize_environment(options.pass_env, options.custom_env);
    env_map.insert("AGENT_SANDBOX_ACTIVE".to_string(), "1".to_string());
    env_map.insert("AGENT_SANDBOX_WORKSPACE".to_string(), options.workspace.to_string_lossy().to_string());
    env_map.insert("TMPDIR".to_string(), temp_guard.session_dir.to_string_lossy().to_string());
    env_map.insert("ZDOTDIR".to_string(), temp_guard.session_dir.to_string_lossy().to_string());
    env_map.insert("HISTFILE".to_string(), temp_guard.session_dir.join(".zsh_history").to_string_lossy().to_string());
    env_map.insert("PROMPT".to_string(), "[sandbox] %~ %# ".to_string());

    let (_c_env_strings, c_env_ptrs) = to_c_env(&env_map);
    let workspace_c = CString::new(options.workspace.to_string_lossy().as_bytes())?;

    // 3. Register parent signal handlers for process group reaping
    unsafe {
        libc::signal(libc::SIGINT, parent_signal_handler as *const () as usize);
        libc::signal(libc::SIGTERM, parent_signal_handler as *const () as usize);
    }

    let is_interactive = options.is_interactive_shell;
    let profile_string = options.profile_content.to_string();

    // 4. Fork child process
    let pid = unsafe { libc::fork() };
    if pid < 0 {
        bail!("fork() failed: {}", std::io::Error::last_os_error());
    }

    if pid == 0 {
        // ================= CHILD PROCESS =================
        unsafe {
            // A. Create new process group
            libc::setpgid(0, 0);

            // B. If interactive shell, claim controlling terminal
            if is_interactive && libc::isatty(0) == 1 {
                libc::tcsetpgrp(0, libc::getpid());
            }

            // C. Bump resource limits (file descriptors to 10240)
            let mut rlim = libc::rlimit { rlim_cur: 0, rlim_max: 0 };
            if libc::getrlimit(libc::RLIMIT_NOFILE, &mut rlim) == 0 {
                let target = if rlim.rlim_max > 0 { std::cmp::min(10240, rlim.rlim_max) } else { 10240 };
                rlim.rlim_cur = target;
                let _ = libc::setrlimit(libc::RLIMIT_NOFILE, &rlim);
            }

            // D. Apply in-memory Apple Seatbelt sandbox
            if let Err(e) = apply_sandbox_profile(&profile_string) {
                eprintln!("[agent-sandbox fatal] Failed to apply kernel sandbox: {}", e);
                libc::_exit(126);
            }

            // E. Change to workspace directory
            if libc::chdir(workspace_c.as_ptr()) != 0 {
                eprintln!("[agent-sandbox fatal] chdir to workspace failed");
                libc::_exit(125);
            }

            // F. Execute binary via execve
            libc::execve(bin_c.as_ptr(), c_arg_ptrs.as_ptr(), c_env_ptrs.as_ptr());

            // If execve returns, an error occurred
            let err = std::io::Error::last_os_error();
            eprintln!("[agent-sandbox fatal] execve failed for {}: {}", resolved_bin.display(), err);
            libc::_exit(127);
        }
    }

    // ================= PARENT PROCESS =================
    ACTIVE_CHILD_PGID.store(pid, Ordering::SeqCst);

    let mut status: libc::c_int = 0;
    let res = unsafe { libc::waitpid(pid, &mut status, 0) };

    // Reset signal handlers and active pgid
    ACTIVE_CHILD_PGID.store(0, Ordering::SeqCst);

    // RAII Reaper: Ensure any orphaned child daemons in the process group are terminated
    unsafe {
        let _ = libc::killpg(pid, libc::SIGTERM);
        if is_interactive && libc::isatty(0) == 1 {
            // Restore terminal control to parent shell
            libc::tcsetpgrp(0, libc::getpgrp());
        }
    }

    if res < 0 {
        bail!("waitpid() failed: {}", std::io::Error::last_os_error());
    }

    // Decode exit status
    if libc::WIFEXITED(status) {
        Ok(libc::WEXITSTATUS(status))
    } else if libc::WIFSIGNALED(status) {
        Ok(128 + libc::WTERMSIG(status))
    } else {
        Ok(1)
    }
}

extern "C" fn parent_signal_handler(sig: libc::c_int) {
    let pgid = ACTIVE_CHILD_PGID.load(Ordering::SeqCst);
    if pgid > 0 {
        unsafe {
            let _ = libc::killpg(pgid, sig);
        }
    }
}
