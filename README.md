# xtask-watch

This crate provides a [`Watch`] that launch a given command, re-launching
the command when changes are detected in your source code.

This [`Watch`] is based on the intended to be used on projects that rely on
the [xtask concept](https://github.com/matklad/cargo-xtask/) and implement
[`clap::Parser`] to be added easily to an existing CLI implementation.

## Setup

The best way to add xtask-watch to your project is to create a workspace
with two packages: your project's package and the xtask package.

### Create a project using xtask

* Create a new directory that will contains the two package of your project
    and the workspace's `Cargo.toml`
    ```console
    mkdir my-project
    cd my-project
    touch Cargo.toml
    ```
* Create the project package and the xtask package using `cargo new`:
    ```console
    cargo new my-project
    cargo new xtask
    ```

* Open the workspace's Cargo.toml and add the following:
    ```toml
    [workspace]
    members = [
        "my-project",
        "xtask",
    ]
    ```

### Add a command alias

Create a `.cargo/config.toml` file and add the following content:

```toml
[alias]
xtask = "run --package xtask --"
```

Now you can run your xtask package using:

```console
cargo xtask
```

### Directory layout example

If the name of the project package is `my-project`, the directory layout should
look like this:

```console
project
├── .cargo
│   └── config.toml
├── Cargo.toml
├── my-project
│   ├── Cargo.toml
│   └── src
│       └── ...
└── xtask
    ├── Cargo.toml
    └── src
        └── main.rs
```

You can find more informations about xtask
[here](https://github.com/cargo-xtask/).

### Use xtask-watch as a dependency

Finally, add the following to the xtask package's Cargo.toml:

```toml
[dependencies]
xtask-watch = "0.1.0"
```

## Examples

* A basic implementation could look like this:

    ```rust,no_run
    use std::process::Command;
    use xtask_watch::{
        anyhow::Result,
        clap,
    };

    #[derive(clap::Parser)]
    enum Opt {
        Watch(xtask_watch::Watch),
    }

    fn main() -> Result<()> {
        let opt: Opt = clap::Parser::parse();

        let mut run_command = Command::new("cargo");
        run_command.arg("check");

        match opt {
            Opt::Watch(watch) => {
                log::info!("Starting to watch `cargo check`");
                watch.run(run_command)?;
            }
        }

        Ok(())
    }
    ```

* [`examples/demo`](https://github.com/rustminded/xtask-watch/tree/main/examples/demo)
    provides an implementation of xtask-watch that naively parse a command
    given by the user (or use `cargo check` by default) and watch the
    workspace after launching this command.

License: MIT OR Apache-2.0
