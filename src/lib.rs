//! This crate provides a [`Watch`](crate::Watch) that launch a given command,
//! re-launching the command when changes are detected in your source code.
//!
//! This [`Watch`](crate::Watch) struct is intended to be used with the
//! [xtask concept](https://github.com/matklad/cargo-xtask/) and implements
//! [`clap::Parser`](https://docs.rs/clap/3.0.14/clap/trait.Parser.html) so it
//! can easily be used in your xtask crate. See [clap's `flatten`](https://github.com/clap-rs/clap/blob/v3.0.14/examples/derive_ref/README.md#arg-attributes)
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
//! xtask-watch = "0.1.0"
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
use lazy_static::lazy_static;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::{
    env,
    path::{Path, PathBuf},
    process::Command,
    sync::mpsc,
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
#[non_exhaustive]
#[derive(Debug, Default, Parser)]
#[clap(about = "Watches over your project's source code.")]
pub struct Watch {
    /// Watch specific file(s) or folder(s).
    ///
    /// The default is the workspace root.
    #[clap(long = "watch", short = 'w')]
    pub watch_paths: Vec<PathBuf>,
    /// Paths that will be excluded.
    #[clap(long = "ignore", short = 'i')]
    pub exclude_paths: Vec<PathBuf>,
    /// Paths, relative to the workspace root, that will be excluded.
    #[clap(skip)]
    pub workspace_exclude_paths: Vec<PathBuf>,
    /// Throttle events to prevent the command to be re-executed too early
    /// right after an execution already occurred.
    ///
    /// The default is 2 seconds.
    #[clap(skip = Duration::from_secs(2))]
    pub debounce: Duration,
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

    /// Set the debounce duration after relaunching the command.
    pub fn debounce(mut self, duration: Duration) -> Self {
        self.debounce = duration;
        self
    }

    /// Run the given `command`, monitor the watched paths and relaunch the
    /// command when changes are detected.
    ///
    /// Workspace's `target` directory and hidden paths are excluded by default.
    pub fn run(mut self, mut command: Command) -> Result<()> {
        let metadata = metadata();

        self.exclude_paths
            .push(metadata.target_directory.clone().into_std_path_buf());

        self.exclude_paths = self
            .exclude_paths
            .into_iter()
            .map(|x| {
                x.canonicalize()
                    .with_context(|| format!("can't find {}", x.display()))
            })
            .collect::<Result<Vec<_>, _>>()?;

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

        let mut child = command.spawn().context("cannot spawn command")?;
        let mut command_start = Instant::now();

        let mut watcher = notify::recommended_watcher(|res| match res {
            Ok(event)
                if !self.is_excluded_path(&path)
                    && path.exists()
                    && !self.is_hidden_path(&path)
                    && !self.is_backup_file(&path)
                    && op != notify::Op::CREATE
                    && op != notify::Op::RENAME
                    && command_start.elapsed() >= self.debounce =>
            {
                log::trace!("Detected changes at {} | {:?}", path.display(), op);
                #[cfg(unix)]
                {
                    let now = Instant::now();

                    unsafe {
                        log::trace!("Killing watch's command process");
                        libc::kill(
                            child.id().try_into().expect("cannot get process id"),
                            libc::SIGTERM,
                        );
                    }

                    while now.elapsed().as_secs() < 2 {
                        std::thread::sleep(Duration::from_millis(200));
                        if let Ok(Some(_)) = child.try_wait() {
                            break;
                        }
                    }
                }

                match child.try_wait() {
                    Ok(Some(_)) => {}
                    _ => {
                        let _ = child.kill();
                        let _ = child.wait();
                    }
                }

                log::info!("Re-running command");
                child = command.spawn().context("cannot spawn command")?;
                command_start = Instant::now();
            }
            Ok(event) => log::trace!("Ignoring changes in {:?}", event),
            Err(err) => log::error!("watch error: {}", err),
        })
        .context("could not initialize watcher")?;

        for path in &self.watch_paths {
            match watcher.watch(&path, RecursiveMode::Recursive) {
                Ok(()) => log::trace!("Watching {}", path.display()),
                Err(err) => log::error!("cannot watch {}: {}", path.display(), err),
            }
        }

        Ok(())
    }

    fn is_excluded_path(&self, path: &Path) -> bool {
        if self.exclude_paths.iter().any(|x| path.starts_with(x)) {
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
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn exclude_relative_path() {
        let watch = Watch {
            debounce: Default::default(),
            watch_paths: Vec::new(),
            exclude_paths: Vec::new(),
            workspace_exclude_paths: vec![PathBuf::from("src/watch.rs")],
        };

        assert!(watch.is_excluded_path(
            metadata()
                .workspace_root
                .join("src")
                .join("watch.rs")
                .as_std_path()
        ));
        assert!(!watch.is_excluded_path(metadata().workspace_root.join("src").as_std_path()));
    }
}
