# luhproc

[<img alt="github" src="https://img.shields.io/badge/github-calizoots/luhproc-8da0cb?style=for-the-badge&labelColor=555555&logo=github" height="20">](https://github.com/calizoots/luhproc)
[<img alt="crates.io" src="https://img.shields.io/crates/v/luhproc.svg?style=for-the-badge&color=fc8d62&logo=rust" height="20">](https://crates.io/crates/luhproc)
[<img alt="docs.rs" src="https://img.shields.io/badge/docs.rs-luhproc-66c2a5?style=for-the-badge&labelColor=555555&logo=docs.rs" height="20">](https://docs.rs/luhproc)

A lightweight background process manager
<br>
> made with love s.c

`luhproc` provides a simple system for spawning, tracking, and stopping
background worker processes in Rust applications.

It is built around two core ideas:
- **Workers are identified by environment variables** (e.g. `MY_TASK=1`)
- **Workers run in background mode when started from the same binary**

This makes it easy to run small persistent tasks (indexers, watchers,
refreshers, schedulers, etc.) without needing systemd, Docker, or any
external supervisor.

## Quick Start

Add `luhtwin` to your `Cargo.toml`:
```toml
[dependencies]
luhproc = "0.1"
```

### Example

```rust

static PM: OnceLock<ProcessManager> = OnceLock::new();

fn child_work() -> LuhTwin<()> {
    info!("hello from the child thread");
    Ok(())
}

fn main() -> LuhTwin<() {
    PM.set(process_manager!("MOTHAPP" => start_moth)
           .encase(|| "failed to make process_manager")?)
        .unwrap();

    PM.get().unwrap().check()
        .encase(|| "failed to run child process")?;

    PM.get().unwrap().start("MOTHAPP", "app", None)
    .encase(|| "failed to start moth app")?;
}
```

---

## Basic Concept

You define tasks using the [`process_manager!`] macro:

```rust
process_manager! {
    "MY_TASK" => my_background_function,
    "REFRESH_CACHE" => refresh_cache_worker,
};
```

A task is triggered when the binary starts **with that environment variable set**.

For example:

```sh
MY_TASK=1 ./myapp
```

The main process will immediately run the function associated with the task,
**then exit after finishing**.

The task can be launched in the background through the API:

```rust
pm.start("MY_TASK", "unique-id", None)?;
```

Each running worker is stored inside a temporary directory containing:

```txt
<tmp>/luhproc/my-task-<hash>/
├── out.log   # stdout
├── err.log   # stderr
└── pid       # process ID
```

---

# Architecture

## `ChildTask`

Represents one runnable background task.

```rust
pub struct ChildTask {
    pub id: String,
    pub env_var: &'static str,
    pub work: fn() -> LuhTwin<()>,
}
```

- `id` — unique instance identifier (affects directory hashing)
- `env_var` — environment variable that triggers the worker
- `work` — the task function itself

### Generated Directory Name

Each task instance gets its own hashed directory:

```txt
my-task-3fa92k19cd12
```

This allows multiple instances of the same task type.

---

## `ProcessManager`

Main API for controlling worker tasks.

```rust
pub struct ProcessManager {
    pub tasks: Vec<ChildTask>,
}
```

### Registering Tasks

```rust
let mut pm = ProcessManager::new()?;
pm.register_task("MY_TASK", my_worker_fn);
```

Or with the macro:

```rust
let pm = process_manager! {
    "MY_TASK" => my_worker_fn,
    "SYNC" => sync_worker_fn,
}?;
```

### Starting a Worker

Spawns a background process by re-invoking the binary with an env var:

```rust
pm.start("MY_TASK", "session42", None)?;
```

Internally:
- creates temp directory  
- forks current executable  
- writes PID file  
- redirects stdout/stderr  

### Stopping a Worker

```rust
pm.stop("MY_TASK", Some("session42"))?;
```

Or stop *all* workers of that type:

```rust
pm.stop("MY_TASK", None)?;
```

### Checking Worker Status

```rust
let details = pm.info("MY_TASK", "session42")?;
println!("{details}");
```

Example output:

```txt
task: MY_TASK (id: session42)
pid: 39241
directory: /tmp/luhproc/my-task-a8fd93c2e1a3
log file: /tmp/luhproc/.../out.log
error file: /tmp/luhproc/.../err.log
```

---

# File Layout

```
/tmp/luhproc/
/
/my-task-ae92f139ab23/
/    pid        # process ID
/    out.log    # stdout of worker
/    err.log    # stderr of worker
```

If `LUHPROC_TMP_DIR` is set, that directory is used instead of `/tmp` or dev temp.

---

#  Environment Trigger Check

At the start of your application, call:

```rust
pm.check()?;
```

If the process was started as a worker, the task runs inline and the parent
process exits afterwards.

Your `main` typically looks like:

```rust
fn main() -> LuhTwin<()> {
    let pm = process_manager! {
        "MY_TASK" => run_worker,
    }?;

    // check for background-worker mode
    pm.check()?;

    // normal application logic
    run_cli()?;
    Ok(())
}
```

---

# Behavior Notes

- Workers are single-process, not threadpools.  
- Stopping uses `SIGTERM` (Unix-only via `nix`).  
- If a PID file becomes invalid, `stop()` will report an error but still clean up.  
- If your worker loops forever, ensure it handles SIGTERM gracefully.  

---

# Example Worker

```rust
fn ping_worker() -> LuhTwin<()> {
    loop {
        println!("ping!");
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}
```

And launching it:

```rust
let pm = process_manager! {
    "PING_WORKER" => ping_worker,
}?;

pm.check()?;


pm.start("PING_WORKER", "main", None)?;
```
