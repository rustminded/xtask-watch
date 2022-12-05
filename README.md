# xtask-watch

[![actions status][actions-badge]][actions-url]
[![crate version][crates-version-badge]][crates-url]
[![documentation][docs-badge]][docs-url]
[![dependencies status][deps-badge]][deps-url]
![licenses][licenses-badge]

[actions-badge]: https://github.com/rustminded/xtask-watch/workflows/main/badge.svg
[actions-url]: https://github.com/rustminded/xtask-watch/actions
[crates-version-badge]: https://img.shields.io/crates/v/xtask-watch
[crates-url]: https://crates.io/crates/xtask-watch
[docs-badge]: https://docs.rs/xtask-watch/badge.svg
[docs-url]: https://docs.rs/xtask-watch/
[deps-badge]: https://deps.rs/crate/xtask-watch/0.1.6/status.svg
[deps-url]: https://deps.rs/crate/xtask-watch
[licenses-badge]: https://img.shields.io/crates/l/xtask-watch

<!-- cargo-rdme start -->

This crate provides a [`Watch`](https://docs.rs/xtask-watch/latest/xtask_watch/struct.Watch.html) that launch a given command, re-launching the
command when changes are detected in your source code.

This [`Watch`](https://docs.rs/xtask-watch/latest/xtask_watch/struct.Watch.html) struct is intended to be used with the
[xtask concept](https://github.com/matklad/cargo-xtask/) and implements
[`clap::Parser`](https://docs.rs/clap/3.0.14/clap/trait.Parser.html) so it can easily be used in
your xtask crate. See [clap's `flatten`](https://github.com/clap-rs/clap/blob/master/examples/derive_ref/flatten_hand_args.rs)
to see how to extend it.

## Setup

The best way to add xtask-watch to your project is to create a workspace with two packages:
your project's package and the xtask package.

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


* Create a `.cargo/config.toml` file and add the following content:

  ```toml
  [alias]
  xtask = "run --package xtask --"
  ```

The directory layout should look like this:

```console
my-project
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

And now you can run your xtask package using:

```console
cargo xtask
```
You can find more informations about xtask
[here](https://github.com/matklad/cargo-xtask/).

### Use xtask-watch as a dependency

Finally, add the following to the xtask package's Cargo.toml:

```toml
[dependencies]
xtask-watch = "0.1.0"
```

## Examples

### A basic implementation

```rust
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

### A more complex demonstration

[`examples/demo`](https://github.com/rustminded/xtask-watch/tree/main/examples/demo) provides an
implementation of xtask-watch that naively parse a command given by the user
(or use `cargo check` by default) and watch the workspace after launching this command.

## Troubleshooting

When using the re-export of [`clap`](https://docs.rs/clap/latest/clap), you
might encounter this error:

```console
error[E0433]: failed to resolve: use of undeclared crate or module `clap`
 --> xtask/src/main.rs:4:10
  |
4 | #[derive(Parser)]
  |          ^^^^^^ use of undeclared crate or module `clap`
  |
  = note: this error originates in the derive macro `Parser` (in Nightly builds, run with -Z macro-backtrace for more info)
```

This occurs because you need to import clap in the scope too. This error can
be resolved like this:

```rust
use xtask_wasm::clap;

#[derive(clap::Parser)]
```

Or like this:

```rust
use xtask_wasm::{clap, clap::Parser};

#[derive(Parser)]
```

<!-- cargo-rdme end -->
