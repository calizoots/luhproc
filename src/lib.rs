//! [![github]](https://github.com/calizoots/luhproc)&ensp;[![crates-io]](https://crates.io/crates/luhproc)&ensp;[![docs-rs]](https://docs.rs/luhproc)
//!
//! [github]: https://img.shields.io/badge/github-calizoots/luhproc-8da0cb?style=for-the-badge&labelColor=555555&logo=github
//! [crates-io]: https://img.shields.io/crates/v/luhproc.svg?style=for-the-badge&color=fc8d62&logo=rust
//! [docs-rs]: https://img.shields.io/badge/docs.rs-luhproc-66c2a5?style=for-the-badge&labelColor=555555&logo=docs.rs
//!
//! # luhproc
//! 
//! A lightweight background process manager
//!
//! > made with love s.c
//!
//! `luhproc` provides a simple system for spawning, tracking, and stopping
//! background worker processes in Rust applications.  
//!
//! It is built around two core ideas:
//! - **Workers are identified by environment variables** (e.g. `MY_TASK=1`)
//! - **Workers run in background mode when started from the same binary**
//!
//! This makes it easy to run small persistent tasks (indexers, watchers,
//! refreshers, schedulers, etc.) without needing systemd, Docker, or any
//! external supervisor.
//!
//! ---
//!
//! ## Quick Start
//! 
//! Add `luhtwin` to your `Cargo.toml`:
//! ```toml
//! [dependencies]
//! luhproc = "0.0.1"
//! ```
//! 
//! ### Example
//! 
//! ```ignore
//! static PM: OnceLock<ProcessManager> = OnceLock::new();
//! 
//! fn child_work() -> LuhTwin<()> {
//!     info!("hello from the child thread");
//!     Ok(())
//! }
//! 
//! fn main() -> LuhTwin<() {
//!     PM.set(process_manager!("MOTHAPP" => start_moth)
//!            .encase(|| "failed to make process_manager")?)
//!         .unwrap();
//! 
//!     PM.get().unwrap().check()
//!         .encase(|| "failed to run child process")?;
//! 
//!     PM.get().unwrap().start("MOTHAPP", "app", None)
//!     .encase(|| "failed to start moth app")?;
//! }
//! ```
//!
//! ---
//!
//! ## Basic Concept
//!
//! You define tasks using the [`process_manager!`] macro:
//!
//! ```ignore
//! process_manager! {
//!     "MY_TASK" => my_background_function,
//!     "REFRESH_CACHE" => refresh_cache_worker,
//! };
//! ```
//!
//! A task is triggered when the binary starts **with that environment variable set**.
//!
//! For example:
//!
//! ```sh
//! MY_TASK=1 ./myapp
//! ```
//!
//! The main process will immediately run the function associated with the task,
//! **then exit after finishing**.
//!
//! The task can be launched in the background through the API:
//!
//! ```ignore
//! pm.start("MY_TASK", "unique-id", None)?;
//! ```
//!
//! Each running worker is stored inside a temporary directory containing:
//!
//! ```txt
//! <tmp>/luhproc/my-task-<hash>/
//! ├── out.log   # stdout
//! ├── err.log   # stderr
//! └── pid       # process ID
//! ```
//!
//! ---
//!
//! # Architecture
//!
//! ## `ChildTask`
//!
//! Represents one runnable background task.
//!
//! ```ignore
//! pub struct ChildTask {
//!     pub id: String,
//!     pub env_var: &'static str,
//!     pub work: fn() -> LuhTwin<()>,
//! }
//! ```
//!
//! - `id` — unique instance identifier (affects directory hashing)
//! - `env_var` — environment variable that triggers the worker
//! - `work` — the task function itself
//!
//! ### Generated Directory Name
//!
//! Each task instance gets its own hashed directory:
//!
//! ```txt
//! my-task-3fa92k19cd12
//! ```
//!
//! This allows multiple instances of the same task type.
//!
//! ---
//!
//! ## `ProcessManager`
//!
//! Main API for controlling worker tasks.
//!
//! ```ignore
//! pub struct ProcessManager {
//!     pub tasks: Vec<ChildTask>,
//! }
//! ```
//!
//! ### Registering Tasks
//!
//! ```ignore
//! let mut pm = ProcessManager::new()?;
//! pm.register_task("MY_TASK", my_worker_fn);
//! ```
//!
//! Or with the macro:
//!
//! ```ignore
//! let pm = process_manager! {
//!     "MY_TASK" => my_worker_fn,
//!     "SYNC" => sync_worker_fn,
//! }?;
//! ```
//!
//! ### Starting a Worker
//!
//! Spawns a background process by re-invoking the binary with an env var:
//!
//! ```ignore
//! pm.start("MY_TASK", "session42", None)?;
//! ```
//!
//! Internally:
//! - creates temp directory  
//! - forks current executable  
//! - writes PID file  
//! - redirects stdout/stderr  
//!
//! ### Stopping a Worker
//!
//! ```ignore
//! pm.stop("MY_TASK", Some("session42"))?;
//! ```
//!
//! Or stop *all* workers of that type:
//!
//! ```ignore
//! pm.stop("MY_TASK", None)?;
//! ```
//!
//! ### Checking Worker Status
//!
//! ```ignore
//! let details = pm.info("MY_TASK", "session42")?;
//! println!("{details}");
//! ```
//!
//! Example output:
//!
//! ```txt
//! task: MY_TASK (id: session42)
//! pid: 39241
//! directory: /tmp/luhproc/my-task-a8fd93c2e1a3
//! log file: /tmp/luhproc/.../out.log
//! error file: /tmp/luhproc/.../err.log
//! ```
//!
//! ---
//!
//! # File Layout
//!
//! ```txt
//! /tmp/luhproc/
//! /
//! /my-task-ae92f139ab23/
//! /    pid        # process ID
//! /    out.log    # stdout of worker
//! /    err.log    # stderr of worker
//! ```
//!
//! If `LUHPROC_TMP_DIR` is set, that directory is used instead of `/tmp` or dev temp.
//!
//! ---
//!
//! #  Environment Trigger Check
//!
//! At the start of your application, call:
//!
//! ```ignore
//! pm.check()?;
//! ```
//!
//! If the process was started as a worker, the task runs inline and the parent
//! process exits afterwards.
//!
//! Your `main` typically looks like:
//!
//! ```ignore
//! fn main() -> LuhTwin<()> {
//!     let pm = process_manager! {
//!         "MY_TASK" => run_worker,
//!     }?;
//!
//!     // check for background-worker mode
//!     pm.check()?;
//!
//!     // normal application logic
//!     run_cli()?;
//!     Ok(())
//! }
//! ```
//!
//! ---
//!
//! # Behavior Notes
//!
//! - Workers are single-process, not threadpools.  
//! - Stopping uses `SIGTERM` (Unix-only via `nix`).  
//! - If a PID file becomes invalid, `stop()` will report an error but still clean up.  
//! - If your worker loops forever, ensure it handles SIGTERM gracefully.  
//!
//! ---
//!
//! # Example Worker
//!
//! ```ignore
//! fn ping_worker() -> LuhTwin<()> {
//!     loop {
//!         println!("ping!");
//!         std::thread::sleep(std::time::Duration::from_secs(1));
//!     }
//! }
//! ```
//!
//! And launching it:
//!
//! ```ignore
//! let pm = process_manager! {
//!     "PING_WORKER" => ping_worker,
//! }?;
//!
//! pm.check()?;
//!
//!
//! pm.start("PING_WORKER", "main", None)?;
//! ```

#[cfg(test)]
mod tests;

use std::env;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::hash::{Hash, Hasher};
use std::collections::HashMap;
use std::process::{Command, Stdio};
use std::collections::hash_map::DefaultHasher;
use std::fs::{read_to_string, remove_file, File};

use luhtwin::at;
use luhtwin::AnyError;
use luhtwin::Encase;
use luhtwin::Wrap;
use nix::unistd::Pid;
use nix::sys::signal::{kill, Signal};
use chrono::Local;

use luhcore::dirs::dev_temp_dir;
use luhlog::{error, info};
use luhtwin::{bail, LuhTwin};

/// Just a global PathBuf for the TMP_DIR
pub static TMP_DIR: OnceLock<PathBuf> = OnceLock::new();

/// Sets the TMP_DIR to
/// - the value of LUHPROC_TMP_DIR if set
/// - if not that in debug assertions it will map to /tmp/ always
/// - if not debug assertions then just std::env::temp_dir().push("luhproc")
fn tmp_dir() -> &'static PathBuf {
    TMP_DIR.get_or_init(|| {
        if let Ok(p) = std::env::var("LUHPROC_TMP_DIR") {
            return PathBuf::from(p);
        }

        if cfg!(debug_assertions) {
            let mut d = dev_temp_dir();
            d.push("luhproc");
            return d 
        }

        let mut d = std::env::temp_dir();
        d.push("luhproc");
        d
    })
}

/// Converts a `u64` value into a lowercase base-36 string.
///
/// This is used to generate short, human-friendly directory hashes
/// for worker instances (e.g. `"3fa92k19cd12"`).
///
/// # Examples
/// ```ignore
/// assert_eq!(to_base36(35), "z");
/// assert_eq!(to_base36(36), "10");
/// ```
fn to_base36(num: u64) -> String {
    let mut n = num;
	let mut out = String::new();
	let alphabet = b"0123456789abcdefghijklmnopqrstuvwxyz";
	while n > 0 {
        out.push(alphabet[(n % 36) as usize] as char);
	    n /= 36;
	}

    if out.is_empty() {
        out.push('0');
	}

    out.chars().rev().collect()
}

pub type ChildWorkFn = fn() -> LuhTwin<()>;

/// Represents a background task that can be launched by the `ProcessManager`.
///
/// A task is triggered by an environment variable (e.g. `MY_TASK=1`) and runs
/// the associated function inside a forked background process.
///
/// Each task instance is uniquely identified by `id`, which is used when hashing
/// the directory name for logs/PID files.
#[derive(Debug, Clone)]
pub struct ChildTask {
    /// # THIS IS DYNAMIC *could change to whatever is identifying for you* used for hashing
    pub id: String,
    /// the env var to check for in ProcessManager::check()
    pub env_var: &'static str,
    /// the handler for this ChildTask
    pub work: ChildWorkFn
}

/// just here for refactoring ease
const PID_FILE_NAME: &str = "pid";

impl ChildTask {
    /// Returns the lowercase, hyphenated base name for this task,
    /// derived from the environment variable.
    ///
    /// For example:
    /// - `MY_TASK` → `"my-task"`
    fn task_name_base(&self) -> String {
        self.env_var.to_lowercase().replace('_', "-")
    }

    /// Returns the full directory name for this task instance, including
    /// the 12-character base-36 hash derived from the task `id`.
    ///
    /// Example:
    /// ```text
    /// my-task-a1b2c3d4e5f6
    /// ```
    ///
    /// # Panics
    /// Panics if `id` is empty. Use `set_id()` when creating tasks.
    fn task_name(&self) -> String {
        if self.id.is_empty() {
            panic!("task id must be set before calling task_name... task: '{}'", self.id);
        }

        let base_name = self.env_var.to_lowercase().replace('_', "-");

        let mut hasher = DefaultHasher::new();
        self.id.hash(&mut hasher);
        let h = hasher.finish();

        let hash_str = to_base36(h)[..12].to_string();

        return format!("{}-{}", base_name, hash_str)
    }

    /// Returns a cloned version of this `ChildTask` with a new instance ID.
    ///
    /// The original task is not modified.
    ///
    /// # Examples
    /// ```ignore
    /// let t2 = task.set_id("worker-42");
    /// assert_eq!(t2.id, "worker-42");
    /// ```
    fn set_id(&self, new_id: impl Into<String>) -> Self {
        let mut cloned = self.clone();
        cloned.id = new_id.into();
        return cloned
    }

    /// Ensures the task's directory exists inside the configured temp directory.
    ///
    /// The directory path is based on the task name hash:
    /// `<tmp>/luhproc/<task-name-hash>/`
    ///
    /// Returns the directory path.
    fn ensure_dirs(&self) -> LuhTwin<PathBuf> {
        let mut dir = tmp_dir().clone();
        dir.push(self.task_name());
        std::fs::create_dir_all(&dir)
            .wrap(|| "failed to create temp dir")?;
        Ok(dir)
    }

    /// Returns the path to the worker's `out.log`.
    fn get_log_file(&self) -> PathBuf {
        let mut path = self.ensure_dirs().unwrap();
        path.push("out.log");
        path
    }

    /// Returns the path to the worker's `err.log`.
    fn get_err_file(&self) -> PathBuf {
        let mut path = self.ensure_dirs().unwrap();
        path.push("err.log");
        path
    }

    /// Returns the path to the worker's PID file.
    fn get_pid_file(&self) -> PathBuf {
        let mut path = self.ensure_dirs().unwrap();
        path.push(PID_FILE_NAME);
        path
    }

    /// Removes the PID file for this worker instance.
    #[allow(dead_code)]
    fn remove_pid_file(&self) -> LuhTwin<()> {
        remove_file(self.get_pid_file().as_path())
            .wrap(|| "failed to remove pid file")?;

        Ok(())
    }
}

/// Central controller responsible for registering tasks, launching
/// background workers, inspecting them, and stopping them.
///
/// Usually created through the `process_manager!` macro.
#[derive(Debug, Clone)]
pub struct ProcessManager {
    pub tasks: Vec<ChildTask>
}

impl ProcessManager {
    fn ensure_dir(p: &Path) -> LuhTwin<()> {
        if p.is_file() {
            bail!("expected dir but found file at {}", p.display());
        }
        if !p.exists() {
            std::fs::create_dir_all(p)?;
        }
        return Ok(())
    }

    /// Creates a new `ProcessManager` and ensures the temporary directory
    /// for worker storage exists.
    ///
    /// The directory is:
    /// - `$LUHPROC_TMP_DIR` if set
    /// - otherwise `/tmp/luhproc` (or debug temp dir in dev builds)
    pub fn new() -> LuhTwin<Self> {
        ProcessManager::ensure_dir(tmp_dir())
            .wrap(|| "ensure_dir failed")?;

        return Ok(Self { tasks: Vec::new() })
    }

    /// Registers a task using its environment trigger and work function.
    ///
    /// Most applications use the `process_manager!` macro instead.
    pub fn register_task(&mut self, env_var: &'static str, work: ChildWorkFn) {
        self.tasks.push(ChildTask { env_var, work, id: "".to_string() });
    }

    /// Detects whether the current process was started as a background worker.
    ///
    /// If the triggering environment variable is set, this method:
    /// - runs the task function immediately
    /// - logs its start time
    /// - and terminates the process afterwards
    ///
    /// If no task matches, this is a no-op and returns normally.
    ///
    /// Call this at the beginning of `main()`.
    pub fn check(&self) -> LuhTwin<()> {
        for task in &self.tasks {
            if env::var(task.env_var).is_ok() {
                info!("{:-<58}", "");
                let now = Local::now();
                let time = now.format("%d/%m/%Y %H:%M").to_string();
                info!("Started background process '{}' at {}", task.env_var, time);
                info!("{:-<58}", "");
                (task.work)()?;
                std::process::exit(0);
            }
        }
        Ok(())
    }

    /// Starts a background worker process for the given task.
    ///
    /// This works by re-invoking the current executable with the task's
    /// environment variable set, allowing the process to enter worker mode
    /// during `check()`.
    ///
    /// Creates the directory:
    /// ```text
    /// <tmp>/luhproc/<task-name-hash>/
    /// ```
    ///
    /// Inside:
    /// - `pid`      — worker PID
    /// - `out.log`  — stdout of worker
    /// - `err.log`  — stderr of worker
    ///
    /// # Errors
    /// - If the task is unknown
    /// - If the PID file already exists (worker already running)
    /// - If the process fails to spawn
    pub fn start(&self, task_name: &str, id: impl Into<String>, extra_env: Option<HashMap<String, String>>) -> LuhTwin<()> {
        let task = match self.tasks.iter().find(|t| t.env_var == task_name) {
            Some(t) => {
                Ok::<_, AnyError>(t.set_id(id))
            },
            None => {
                return Err(at!("no registered task found with name '{}'", task_name).into())
            }
        }?;

        ChildTask::ensure_dirs(&task)
            .encase(|| format!("failed to create directories for task '{}'", task_name))?;

        if task.get_pid_file().as_path().exists() {
            return Err(at!("process is already running pid file at {}", task.get_pid_file().display()).into())
        }
        
        let exe = env::current_exe()
            .wrap(|| "failed to get the current exe path")?;
        
        let dout_file = File::create(task.get_log_file().as_path());
        let derr_file = File::create(task.get_err_file().as_path());
        
        if dout_file.is_err() || derr_file.is_err() {
            return Err("failed to create out and err files for child process...".into())
        }

        let mut cmd = Command::new(exe);

        cmd.env(task.env_var, task_name);

        if let Some(env_map) = extra_env {
            for (k, v) in env_map {
                cmd.env(k, v);
            }
        }
        
        let child = cmd 
            .stdin(Stdio::null())
            .stdout(dout_file.unwrap())
            .stderr(derr_file.unwrap())
            .spawn()
            .wrap(|| "failed to fork into the background")?;
        
        let mut pidfile = File::create(task.get_pid_file().as_path())
            .wrap(|| "failed to create the pid file use ps aux to shut the child process")?;
        
        write!(pidfile, "{}", child.id())
            .wrap(|| "failed to write to the pid file use ps aux to shut child process")?;
        
        info!("successfully started the background process '{}' with pid {}", task_name, child.id());
        Ok(())
    }

    /// Finds all directories in the temp folder that match this task's base name.
    ///
    /// Used internally for stopping workers without specifying an instance `id`.
    fn find_task_dirs(&self, task: &ChildTask) -> LuhTwin<Vec<PathBuf>> {
        let mut dirs = Vec::new();
        let tmp = tmp_dir();

        if !tmp.exists() { return Ok(dirs); }
        for entry in std::fs::read_dir(tmp)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with(&task.task_name_base()) {
                    dirs.push(entry.path());
                }
            }
        }
        Ok(dirs)
    }

    /// Returns a human-readable summary of the worker instance:
    ///
    /// ```text
    /// task: MY_TASK (id: session42)
    /// pid: 39241
    /// directory: /tmp/luhproc/my-task-a8fd93c2e1a3
    /// log file: ...
    /// error file: ...
    /// ```
    ///
    /// If the worker is not running or has an invalid PID file,
    /// a descriptive message is returned.
    pub fn info(&self, task_name: &str, id: impl Into<String>) -> LuhTwin<String> {
        let task = match self.tasks.iter().find(|t| t.env_var == task_name) {
            Some(t) => Ok::<_, AnyError>(t.set_id(id)),
            None => {
                return Err(at!("no registered task found with name '{}'", task_name).into())
            }
        }?;
        
        let task_dir = tmp_dir().join(task.task_name());
        
        if !task_dir.exists() {
            return Ok(format!("task '{}' (id: '{}') is not running", task_name, task.id));
        }
        
        let pid_file = task.get_pid_file();
        let pid: u32 = match read_to_string(&pid_file).ok().and_then(|s| s.trim().parse().ok()) {
            Some(p) => p,
            None => {
                return Ok(format!("task '{}' (id: '{}') has invalid pid file", task_name, task.id));
            }
        };
        
        let log_file = task.get_log_file();
        let err_file = task.get_err_file();
        
        let mut info = format!("task: {} (id: {})\n", task_name, task.id);
        info.push_str(&format!("pid: {}\n", pid));
        info.push_str(&format!("directory: {}\n", task_dir.display()));
        info.push_str(&format!("log file: {}\n", log_file.display()));
        info.push_str(&format!("error file: {}", err_file.display()));
        
        Ok(info)
    }

    /// Stops a running worker by sending it `SIGTERM`.
    ///
    /// If `id` is given:
    /// - stops only that specific instance
    ///
    /// If `id` is `None`:
    /// - stops *all* workers whose directory names match the task
    ///
    /// After a worker is stopped, its directory is removed.
    ///
    /// # Errors
    /// - If no matching workers are found
    /// - If killing any worker fails
    pub fn stop(&self, task_name: &str, id: Option<impl Into<String> + Clone>) -> LuhTwin<()> {
        let task = match self.tasks.iter().find(|t| t.env_var == task_name) {
            Some(t) => {
                if id.is_some() {
                    Ok::<_, AnyError>(t.set_id(id.clone().unwrap().into()))
                } else {
                    Ok(t.clone())
                }
            },
            None => {
                Err(at!("no registered task found with name '{}'", task_name).into())
            }
        }?;

        if id.is_none() {
            let dirs = self.find_task_dirs(&task)
                .wrap(|| "failed to list task directories")?;

            if dirs.is_empty() {
                return Err(at!("no running processes found for '{}'", task_name).into())
            }

            let mut errors = 0;
            for dir in dirs {
                let pid_file = dir.join(PID_FILE_NAME);
                if !pid_file.exists() {
                    continue;
                }

                let pid: u32 = match read_to_string(&pid_file).ok().and_then(|s| s.trim().parse().ok()) {
                    Some(p) => p,
                    None => {
                        error!("invalid pid file at {}", pid_file.display());
                        errors += 1;
                        continue;
                    }
                };

                if let Err(e) = kill(Pid::from_raw(pid as i32), Signal::SIGTERM) {
                    error!("failed to kill process {}: {}", pid, e);
                    errors += 1;
                } else {
                    info!("stopped process {} at {}", pid, dir.display());
                    let _ = std::fs::remove_dir_all(dir);
                }
            }

            if errors > 0 {
                Err(at!("failed to stop some processes check the log for more details").into())
            } else { Ok(()) }
        } else {
            let task_dir = tmp_dir().join(task.task_name());
            if !task_dir.exists() {
                return Err(at!("no running process found for '{}', id '{}'", task_name, task.id).into())
            }
            let pid: u32 = match read_to_string(task.get_pid_file()).ok().and_then(|s| s.trim().parse().ok()) {
                Some(p) => Ok::<_, AnyError>(p),
                None => {
                    Err(at!("invalid pid file at {}", task.get_pid_file().display()).into())
                }
            }?;

            if let Err(e) = kill(Pid::from_raw(pid as i32), Signal::SIGTERM) {
                let _ = std::fs::remove_dir_all(task_dir);
                Err(at!("failed to kill process {}: {}", pid, e).into())
            } else {
                info!("stopped process {} at {}", pid, task_dir.display());
                let _ = std::fs::remove_dir_all(task_dir);
                Ok(())
            }
        }
    }
}

/// Convenience macro that constructs a `ProcessManager` and registers
/// multiple tasks in a compact syntax.
///
/// # Example
/// ```ignore
/// let pm = process_manager! {
///     "MY_TASK" => run,
///     "SYNC"    => sync_worker,
/// }?;
/// ```
#[macro_export]
macro_rules! process_manager {
    ( $( $env:expr => $func:expr ),* $(,)? ) => {{
        let mut pm = $crate::ProcessManager::new()?;
        $(
            pm.register_task($env, $func);
        )*
        Ok::<_, luhtwin::AnyError>(pm)
    }};
}
