//! This crate provides a [`Watch`](crate::Watch) that launch a given command,
//! re-launching the command when changes are detected in your source code.
//!
//! This [`Watch`](crate::Watch) struct is intended to be used with the
//! [xtask concept](https://github.com/matklad/cargo-xtask/) and implements
//! [`clap::Parser`](https://docs.rs/clap/latest/clap/trait.Parser.html) so it
//! can easily be used in your xtask crate. See [clap's `flatten`](https://github.com/clap-rs/clap/blob/master/examples/derive_ref/flatten_hand_args.rs)
//! to see how to extend it.
//!
//! # Setup
//!
//! The best way to add xtask-watch to your project is to create a workspace
//! with two packages: your project's package and the xtask package.
//!
//! ## Create a project using xtask
//!
//! * Create a new directory that will contains the two package of your project
//!   and the workspace's `Cargo.toml`
//!
//!   ```console
//!   mkdir my-project
//!   cd my-project
//!   touch Cargo.toml
//!   ```
//!
//! * Create the project package and the xtask package using `cargo new`:
//!
//!   ```console
//!   cargo new my-project
//!   cargo new xtask
//!   ```
//!
//! * Open the workspace's Cargo.toml and add the following:
//!
//!   ```toml
//!   [workspace]
//!   members = [
//!       "my-project",
//!       "xtask",
//!   ]
//!   ```
//!
//!
//! * Create a `.cargo/config.toml` file and add the following content:
//!
//!   ```toml
//!   [alias]
//!   xtask = "run --package xtask --"
//!   ```
//!
//! The directory layout should look like this:
//!
//! ```console
//! my-project
//! ├── .cargo
//! │   └── config.toml
//! ├── Cargo.toml
//! ├── my-project
//! │   ├── Cargo.toml
//! │   └── src
//! │       └── ...
//! └── xtask
//!     ├── Cargo.toml
//!     └── src
//!         └── main.rs
//! ```
//!
//! And now you can run your xtask package using:
//!
//! ```console
//! cargo xtask
//! ```
//! You can find more informations about xtask
//! [here](https://github.com/matklad/cargo-xtask/).
//!
//! ## Use xtask-watch as a dependency
//!
//! Finally, add the following to the xtask package's Cargo.toml:
//!
//! ```toml
//! [dependencies]
//! xtask-watch = "0.3"
//! ```
//!
//! # Examples
//!
//! ## A basic implementation
//!
//! ```rust,no_run
//! use std::process::Command;
//! use xtask_watch::{
//!     anyhow::Result,
//!     clap,
//! };
//!
//! #[derive(clap::Parser)]
//! enum Opt {
//!     Watch(xtask_watch::Watch),
//! }
//!
//! fn main() -> Result<()> {
//!     let opt: Opt = clap::Parser::parse();
//!
//!     let mut run_command = Command::new("cargo");
//!     run_command.arg("check");
//!
//!     match opt {
//!         Opt::Watch(watch) => {
//!             log::info!("Starting to watch `cargo check`");
//!             watch.run(run_command)?;
//!         }
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! ## A more complex demonstration
//!
//! [`examples/demo`](https://github.com/rustminded/xtask-watch/tree/main/examples/demo)
//! provides an implementation of xtask-watch that naively parse a command given
//! by the user (or use `cargo check` by default) and watch the workspace after
//! launching this command.
//!
//! # Troubleshooting
//!
//! When using the re-export of [`clap`](https://docs.rs/clap/latest/clap), you
//! might encounter this error:
//!
//! ```console
//! error[E0433]: failed to resolve: use of undeclared crate or module `clap`
//!  --> xtask/src/main.rs:4:10
//!   |
//! 4 | #[derive(Parser)]
//!   |          ^^^^^^ use of undeclared crate or module `clap`
//!   |
//!   = note: this error originates in the derive macro `Parser` (in Nightly builds, run with -Z macro-backtrace for more info)
//! ```
//!
//! This occurs because you need to import clap in the scope too. This error can
//! be resolved like this:
//!
//! ```rust
//! use xtask_watch::clap;
//!
//! #[derive(clap::Parser)]
//! struct MyStruct {}
//! ```
//!
//! Or like this:
//!
//! ```rust
//! use xtask_watch::{clap, clap::Parser};
//!
//! #[derive(Parser)]
//! struct MyStruct {}
//! ```

#![deny(missing_docs)]

use anyhow::{Context, Result};
use clap::Parser;
use glob::Pattern;
use lazy_static::lazy_static;
use notify::Watcher as _;
use std::{
    env, io,
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus},
    sync::{Arc, Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard, mpsc},
    thread,
    time::{Duration, Instant},
};

pub use anyhow;
pub use cargo_metadata;
pub use cargo_metadata::camino;
pub use clap;

/// Fetch the metadata of the crate.
pub fn metadata() -> &'static cargo_metadata::Metadata {
    lazy_static! {
        static ref METADATA: cargo_metadata::Metadata = cargo_metadata::MetadataCommand::new()
            .exec()
            .expect("cannot get crate's metadata");
    }

    &METADATA
}

/// Fetch information of a package in the current crate.
pub fn package(name: &str) -> Option<&cargo_metadata::Package> {
    metadata().packages.iter().find(|x| x.name == name)
}

/// Return a [`std::process::Command`] of the xtask command currently running.
pub fn xtask_command() -> Command {
    Command::new(env::args_os().next().unwrap())
}

/// Watches over your project's source code, relaunching a given command when
/// changes are detected.
///
/// Use [`Watch::lock`] to obtain a [`WatchLock`] that can be shared with external
/// code (e.g. an HTTP server) to coordinate reads with ongoing rebuilds.
#[non_exhaustive]
#[derive(Clone, Debug, Default, Parser)]
#[clap(about = "Watches over your project's source code.")]
pub struct Watch {
    /// Shell command(s) to execute on changes.
    #[clap(long = "shell", short = 's')]
    pub shell_commands: Vec<String>,
    /// Cargo command(s) to execute on changes.
    ///
    /// The default is `[ check ]`
    #[clap(long = "exec", short = 'x')]
    pub cargo_commands: Vec<String>,
    /// Watch specific file(s) or folder(s).
    ///
    /// The default is the workspace root.
    #[clap(long = "watch", short = 'w')]
    pub watch_paths: Vec<PathBuf>,
    /// Paths or glob patterns that will be excluded.
    ///
    /// Relative values are resolved from the current working directory.
    #[clap(long = "ignore", short = 'i')]
    pub exclude_paths: Vec<PathBuf>,
    /// Paths or glob patterns, relative to the workspace root, that will be excluded.
    #[clap(skip)]
    pub workspace_exclude_paths: Vec<PathBuf>,
    /// Throttle events to prevent the command to be re-executed too early
    /// right after an execution already occurred.
    ///
    /// The default is 2 seconds.
    #[clap(skip = Duration::from_secs(2))]
    pub debounce: Duration,
    #[clap(skip)]
    exclude_globs: Vec<Pattern>,
    #[clap(skip)]
    workspace_exclude_globs: Vec<Pattern>,
    #[clap(skip)]
    watch_lock: WatchLock,
}

impl Watch {
    /// Add a path to watch for changes.
    pub fn watch_path(mut self, path: impl AsRef<Path>) -> Self {
        self.watch_paths.push(path.as_ref().to_path_buf());
        self
    }

    /// Add multiple paths to watch for changes.
    pub fn watch_paths(mut self, paths: impl IntoIterator<Item = impl AsRef<Path>>) -> Self {
        for path in paths {
            self.watch_paths.push(path.as_ref().to_path_buf())
        }
        self
    }

    /// Add a path that will be ignored if changes are detected.
    pub fn exclude_path(mut self, path: impl AsRef<Path>) -> Self {
        self.exclude_paths.push(path.as_ref().to_path_buf());
        self
    }

    /// Add multiple paths that will be ignored if changes are detected.
    pub fn exclude_paths(mut self, paths: impl IntoIterator<Item = impl AsRef<Path>>) -> Self {
        for path in paths {
            self.exclude_paths.push(path.as_ref().to_path_buf());
        }
        self
    }

    /// Add a path, relative to the workspace, that will be ignored if changes
    /// are detected.
    pub fn exclude_workspace_path(mut self, path: impl AsRef<Path>) -> Self {
        self.workspace_exclude_paths
            .push(path.as_ref().to_path_buf());
        self
    }

    /// Add multiple paths, relative to the workspace, that will be ignored if
    /// changes are detected.
    pub fn exclude_workspace_paths(
        mut self,
        paths: impl IntoIterator<Item = impl AsRef<Path>>,
    ) -> Self {
        for path in paths {
            self.workspace_exclude_paths
                .push(path.as_ref().to_path_buf());
        }
        self
    }

    /// Return the shared lock used by this watcher.
    ///
    /// Clone and share this lock with external code (e.g. HTTP handlers) to coordinate with
    /// watch-driven command execution.
    ///
    /// # Lock lifecycle
    ///
    /// [`run`](Self::run) acquires the **write lock immediately** when it is called — before the
    /// first command is even spawned. Any code that calls [`WatchLock::acquire`] will therefore
    /// block until the first build completes. This is intentional: it prevents readers from
    /// observing an empty or incomplete dist directory before the initial build has finished.
    /// The write lock is then re-acquired on every subsequent rebuild and released once the
    /// command sequence succeeds.
    #[must_use = "store and share the lock with readers that must coordinate with rebuilds"]
    pub fn lock(&self) -> WatchLock {
        self.watch_lock.clone()
    }

    /// Set the debounce duration after relaunching the command.
    pub fn debounce(mut self, duration: Duration) -> Self {
        self.debounce = duration;
        self
    }

    /// Run the given `command`, monitor the watched paths and relaunch the
    /// command when changes are detected.
    ///
    /// Workspace's `target` directory and hidden paths are excluded by default.
    pub fn run(mut self, commands: impl Into<CommandList>) -> Result<()> {
        let metadata = metadata();
        let list = commands.into();

        {
            let mut commands = list
                .commands
                .lock()
                .expect("no panic-prone code runs while this lock is held");

            commands.extend(self.shell_commands.iter().map(|x| {
                let mut command =
                    Command::new(env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string()));
                command.arg("-c");
                command.arg(x);

                command
            }));

            commands.extend(self.cargo_commands.iter().map(|x| {
                let mut command =
                    Command::new(env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string()));
                command.arg("-c");
                command.arg(format!("cargo {x}"));

                command
            }));
        }

        self.prepare_excludes()?;

        if self.watch_paths.is_empty() {
            self.watch_paths
                .push(metadata.workspace_root.clone().into_std_path_buf());
        }

        self.watch_paths = self
            .watch_paths
            .into_iter()
            .map(|x| {
                x.canonicalize()
                    .with_context(|| format!("can't find {}", x.display()))
            })
            .collect::<Result<Vec<_>, _>>()?;

        let (tx, rx) = mpsc::channel();

        let handler = WatchEventHandler {
            watch: self.clone(),
            tx: tx.clone(),
            command_start: Instant::now(),
        };

        let mut watcher =
            notify::recommended_watcher(handler).context("could not initialize watcher")?;

        for path in &self.watch_paths {
            match watcher.watch(path, notify::RecursiveMode::Recursive) {
                Ok(()) => log::trace!("Watching {}", path.display()),
                Err(err) => log::error!("cannot watch {}: {err}", path.display()),
            }
        }

        let mut current_child = SharedChild::new();
        let mut lock_guard = Some(self.watch_lock.write());
        let mut generation: u64 = 0;
        loop {
            if lock_guard.is_some() {
                log::info!("Running command");
                let mut current_child = current_child.clone();
                let mut list = list.clone();
                let tx = tx.clone();
                let build_id = generation;
                thread::spawn(move || {
                    let mut status = ExitStatus::default();

                    list.spawn(|res| match res {
                        Err(err) => {
                            log::error!("Could not execute command: {err}");
                            false
                        }
                        Ok(child) => {
                            log::trace!("Child spawned PID: {}", child.id());
                            current_child.replace(child);
                            status = current_child.wait();
                            status.success()
                        }
                    });

                    if status.success() {
                        log::info!("Command succeeded.");
                        tx.send(Event::CommandSucceeded(build_id))
                            .expect("can send");
                    } else if let Some(code) = status.code() {
                        log::error!("Command failed (exit code: {code})");
                    } else {
                        log::error!("Command failed.");
                    }
                });
            }

            match rx.recv() {
                Ok(Event::ChangeDetected) => {
                    log::trace!("Changes detected, re-generating");
                    if lock_guard.is_none() {
                        lock_guard = Some(self.watch_lock.write());
                    }
                    generation += 1;
                    current_child.terminate();
                }
                Ok(Event::CommandSucceeded(build_id)) if build_id == generation => {
                    lock_guard.take();
                }
                Ok(Event::CommandSucceeded(build_id)) => {
                    log::trace!(
                        "Ignoring stale success from build {build_id} (current: {generation})"
                    );
                }
                Err(_) => {
                    current_child.terminate();
                    break;
                }
            }
        }

        Ok(())
    }

    fn is_excluded_path(&self, path: &Path) -> bool {
        if self.exclude_paths.iter().any(|x| path.starts_with(x)) {
            return true;
        }

        if self.exclude_globs.iter().any(|p| p.matches_path(path)) {
            return true;
        }

        if let Ok(stripped_path) = path.strip_prefix(metadata().workspace_root.as_std_path()) {
            if self
                .workspace_exclude_paths
                .iter()
                .any(|x| stripped_path.starts_with(x))
            {
                return true;
            }

            if self
                .workspace_exclude_globs
                .iter()
                .any(|p| p.matches_path(stripped_path))
            {
                return true;
            }
        }

        false
    }

    fn is_hidden_path(&self, path: &Path) -> bool {
        self.watch_paths.iter().any(|x| {
            path.strip_prefix(x)
                .iter()
                .any(|x| x.to_string_lossy().starts_with('.'))
        })
    }

    fn is_backup_file(&self, path: &Path) -> bool {
        self.watch_paths.iter().any(|x| {
            path.strip_prefix(x)
                .iter()
                .any(|x| x.to_string_lossy().ends_with('~'))
        })
    }

    fn is_glob_pattern(path: &Path) -> bool {
        let s = path.as_os_str().to_string_lossy();
        s.contains('*') || s.contains('?') || (!cfg!(windows) && s.contains('['))
    }

    fn compile_glob(path: &Path) -> Result<Pattern> {
        let pattern = path
            .to_str()
            .with_context(|| format!("glob pattern must be valid UTF-8: {}", path.display()))?;

        Pattern::new(pattern).with_context(|| format!("invalid glob pattern: `{}`", path.display()))
    }

    fn prepare_excludes(&mut self) -> Result<()> {
        let metadata = metadata();
        self.exclude_paths
            .push(metadata.target_directory.clone().into_std_path_buf());

        let current_dir = env::current_dir().context("failed to get current directory")?;
        let mut exclude_paths = Vec::new();
        for path in self.exclude_paths.iter() {
            if Self::is_glob_pattern(path) {
                let absolute = if path.is_absolute() {
                    path.to_path_buf()
                } else {
                    current_dir.join(path)
                };
                self.exclude_globs.push(Self::compile_glob(&absolute)?);
            } else {
                let canonical = path
                    .canonicalize()
                    .with_context(|| format!("can't find `{}`", path.display()))?;
                exclude_paths.push(canonical);
            }
        }
        self.exclude_paths = exclude_paths;

        let workspace_root = metadata.workspace_root.as_std_path();
        let mut workspace_exclude_paths = Vec::new();
        for path in self.workspace_exclude_paths.iter() {
            let path = if path.is_absolute() {
                path.strip_prefix(workspace_root)
                    .with_context(|| {
                        format!(
                            "workspace exclude path must be inside workspace root: `{}`",
                            path.display()
                        )
                    })?
                    .to_path_buf()
            } else {
                path.to_path_buf()
            };

            if Self::is_glob_pattern(&path) {
                self.workspace_exclude_globs
                    .push(Self::compile_glob(&path)?);
            } else {
                workspace_exclude_paths.push(path);
            }
        }
        self.workspace_exclude_paths = workspace_exclude_paths;

        Ok(())
    }
}

struct WatchEventHandler {
    watch: Watch,
    tx: mpsc::Sender<Event>,
    command_start: Instant,
}

impl notify::EventHandler for WatchEventHandler {
    fn handle_event(&mut self, event: Result<notify::Event, notify::Error>) {
        match event {
            Ok(event) => {
                if (event.kind.is_modify() || event.kind.is_create())
                    && event.paths.iter().any(|x| {
                        !self.watch.is_excluded_path(x)
                            && x.exists()
                            && !self.watch.is_hidden_path(x)
                            && !self.watch.is_backup_file(x)
                            && self.command_start.elapsed() >= self.watch.debounce
                    })
                {
                    log::trace!("Changes detected in {event:?}");
                    self.command_start = Instant::now();

                    self.tx.send(Event::ChangeDetected).expect("can send");
                } else {
                    log::trace!("Ignoring changes in {event:?}");
                }
            }
            Err(err) => log::error!("watch error: {err}"),
        }
    }
}

#[derive(Debug, Clone)]
struct SharedChild {
    child: Arc<Mutex<Option<Child>>>,
}

impl SharedChild {
    fn new() -> Self {
        Self {
            child: Default::default(),
        }
    }

    fn replace(&mut self, child: impl Into<Option<Child>>) {
        *self
            .child
            .lock()
            .expect("no panic-prone code runs while this lock is held") = child.into();
    }

    fn wait(&mut self) -> ExitStatus {
        loop {
            let mut child = self
                .child
                .lock()
                .expect("no panic-prone code runs while this lock is held");
            match child.as_mut().map(|child| child.try_wait()) {
                Some(Ok(Some(status))) => {
                    break status;
                }
                Some(Ok(None)) => {
                    drop(child);
                    thread::sleep(Duration::from_millis(10));
                }
                Some(Err(err)) => {
                    log::error!("could not wait for child process: {err}");
                    break Default::default();
                }
                None => {
                    break Default::default();
                }
            }
        }
    }

    fn terminate(&mut self) {
        if let Some(child) = self
            .child
            .lock()
            .expect("no panic-prone code runs while this lock is held")
            .as_mut()
        {
            #[cfg(unix)]
            {
                let killing_start = Instant::now();

                unsafe {
                    log::trace!("sending SIGTERM to {}", child.id());
                    libc::kill(child.id() as _, libc::SIGTERM);
                }

                while killing_start.elapsed().as_secs() < 2 {
                    std::thread::sleep(Duration::from_millis(200));
                    if let Ok(Some(_)) = child.try_wait() {
                        break;
                    }
                }
            }

            match child.try_wait() {
                Ok(Some(_)) => {}
                _ => {
                    log::trace!("killing {}", child.id());
                    let _ = child.kill();
                    let _ = child.wait();
                }
            }
        } else {
            log::trace!("nothing to terminate");
        }
    }
}

/// A list of commands to run.
#[derive(Debug, Clone)]
pub struct CommandList {
    commands: Arc<Mutex<Vec<Command>>>,
}

impl From<Command> for CommandList {
    fn from(command: Command) -> Self {
        Self {
            commands: Arc::new(Mutex::new(vec![command])),
        }
    }
}

impl From<Vec<Command>> for CommandList {
    fn from(commands: Vec<Command>) -> Self {
        Self {
            commands: Arc::new(Mutex::new(commands)),
        }
    }
}

impl<const SIZE: usize> From<[Command; SIZE]> for CommandList {
    fn from(commands: [Command; SIZE]) -> Self {
        Self {
            commands: Arc::new(Mutex::new(Vec::from(commands))),
        }
    }
}

impl CommandList {
    /// Returns `true` if the list is empty.
    pub fn is_empty(&self) -> bool {
        self.commands
            .lock()
            .expect("no panic-prone code runs while this lock is held")
            .is_empty()
    }

    /// Spawn each command of the list one after the other.
    ///
    /// The caller is responsible to wait the commands.
    pub fn spawn(&mut self, mut callback: impl FnMut(io::Result<Child>) -> bool) {
        for process in self
            .commands
            .lock()
            .expect("no panic-prone code runs while this lock is held")
            .iter_mut()
        {
            if !callback(process.spawn()) {
                break;
            }
        }
    }

    /// Run all the commands sequentially using [`std::process::Command::status`] and stop at the
    /// first failure.
    pub fn status(&mut self) -> io::Result<ExitStatus> {
        for process in self
            .commands
            .lock()
            .expect("no panic-prone code runs while this lock is held")
            .iter_mut()
        {
            let exit_status = process.status()?;
            if !exit_status.success() {
                return Ok(exit_status);
            }
        }
        Ok(Default::default())
    }
}

/// Guard returned by [`WatchLock::acquire`].
///
/// Keep this value alive for the duration of the protected read section.
/// The lock is released automatically when the guard is dropped.
pub struct WatchLockGuard<'a> {
    _guard: RwLockReadGuard<'a, ()>,
}

/// A lock handle used to coordinate file reads with watch-driven rebuilds.
///
/// Obtain it from [`Watch::lock`], clone it, and call [`WatchLock::acquire`] while
/// reading files that must not race with rebuild writes.
#[derive(Clone, Debug, Default)]
pub struct WatchLock(Arc<RwLock<()>>);

impl WatchLock {
    /// Acquire shared access to the protected section.
    ///
    /// Multiple readers may hold this guard concurrently.
    pub fn acquire(&self) -> WatchLockGuard<'_> {
        WatchLockGuard {
            // The inner value is `()` — there is no data to corrupt, so we can
            // always recover from a poisoned lock.
            _guard: self.0.read().unwrap_or_else(|e| e.into_inner()),
        }
    }

    fn write(&self) -> RwLockWriteGuard<'_, ()> {
        // The inner value is `()` — there is no data to corrupt, so we can
        // always recover from a poisoned lock.
        self.0.write().unwrap_or_else(|e| e.into_inner())
    }
}

#[derive(Debug)]
enum Event {
    CommandSucceeded(u64),
    ChangeDetected,
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn exclude_relative_path() {
        let watch = Watch::default().exclude_workspace_path("src/watch.rs");

        assert!(
            watch.is_excluded_path(
                metadata()
                    .workspace_root
                    .join("src")
                    .join("watch.rs")
                    .as_std_path()
            )
        );
        assert!(!watch.is_excluded_path(metadata().workspace_root.join("src").as_std_path()));
    }

    #[test]
    fn exclude_absolute_glob_path() {
        let absolute = metadata()
            .workspace_root
            .join("src")
            .join("**")
            .join("*.rs");

        let mut watch = Watch::default().exclude_path(absolute);
        watch
            .prepare_excludes()
            .expect("exclude parsing should succeed");
        assert_eq!(watch.exclude_globs.len(), 1);

        assert!(
            watch.is_excluded_path(
                metadata()
                    .workspace_root
                    .join("src")
                    .join("lib.rs")
                    .as_std_path()
            )
        );
    }

    #[test]
    fn exclude_workspace_glob_path() {
        let mut watch = Watch::default().exclude_workspace_path("src/**/*.rs");
        watch
            .prepare_excludes()
            .expect("exclude parsing should succeed");
        assert_eq!(watch.workspace_exclude_globs.len(), 1);

        assert!(
            watch.is_excluded_path(
                metadata()
                    .workspace_root
                    .join("src")
                    .join("lib.rs")
                    .as_std_path()
            )
        );
    }

    #[test]
    fn exclude_workspace_absolute_glob_path() {
        let absolute = metadata()
            .workspace_root
            .join("src")
            .join("**")
            .join("*.rs");
        let mut watch = Watch::default().exclude_workspace_path(absolute);
        watch
            .prepare_excludes()
            .expect("exclude parsing should succeed");

        assert_eq!(watch.workspace_exclude_globs.len(), 1);
        assert!(
            watch.is_excluded_path(
                metadata()
                    .workspace_root
                    .join("src")
                    .join("lib.rs")
                    .as_std_path()
            )
        );
    }

    #[test]
    fn exclude_workspace_glob_non_match() {
        let mut watch = Watch::default().exclude_workspace_path("tests/**/*.rs");
        watch
            .prepare_excludes()
            .expect("exclude parsing should succeed");

        assert!(
            !watch.is_excluded_path(
                metadata()
                    .workspace_root
                    .join("src")
                    .join("lib.rs")
                    .as_std_path()
            )
        );
    }

    #[test]
    fn glob_detection() {
        assert!(Watch::is_glob_pattern(Path::new("src/**/*.rs")));
        assert!(Watch::is_glob_pattern(Path::new("foo?.rs")));

        #[cfg(not(windows))]
        assert!(Watch::is_glob_pattern(Path::new("[ab].rs")));
        #[cfg(windows)]
        assert!(!Watch::is_glob_pattern(Path::new("[ab].rs")));

        assert!(!Watch::is_glob_pattern(Path::new("src/lib.rs")));
    }

    #[test]
    fn invalid_glob_pattern() {
        let err = Watch::compile_glob(Path::new("[abc")).expect_err("should fail");
        assert!(
            err.to_string().contains("invalid glob pattern"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn command_list_froms() {
        let _: CommandList = Command::new("foo").into();
        let _: CommandList = vec![Command::new("foo")].into();
        let _: CommandList = [Command::new("foo")].into();
    }
}
