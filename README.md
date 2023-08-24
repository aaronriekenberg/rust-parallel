# rust-parallel

Command-line utility to execute commands in parallel and aggregate their output.

Similar interface to [GNU Parallel](https://www.gnu.org/software/parallel/parallel_examples.html) or [xargs](https://man7.org/linux/man-pages/man1/xargs.1.html) but implemented in rust and [tokio](https://tokio.rs).
* Run commands read from stdin, input files, or `:::` on command line.
* Transform inputs with regular expression capture groups
* TUI progress bar
* Path cache
* Command timeouts
* Structured debug logging
* Prevents [output interleaving](https://github.com/aaronriekenberg/rust-parallel/wiki/Output-Interleaving)
* [Very fast](https://github.com/aaronriekenberg/rust-parallel/wiki/Benchmarks).
 
See [examples](https://github.com/aaronriekenberg/rust-parallel/wiki/Examples) for example commands and [manual](https://github.com/aaronriekenberg/rust-parallel/wiki/Manual) for more details.

Listed in [Awesome Rust - utilities](https://github.com/rust-unofficial/awesome-rust#utilities)

[crates-badge]: https://img.shields.io/crates/v/rust-parallel.svg
[crates-url]: https://crates.io/crates/rust-parallel

[ci-badge]: https://github.com/aaronriekenberg/rust-parallel/actions/workflows/CI.yml/badge.svg
[ci-url]: https://github.com/aaronriekenberg/rust-parallel/actions/workflows/CI.yml 

[![Crates.io][crates-badge]][crates-url] [![CI workflow][ci-badge]][ci-url]

## Contents:
* [Installation](#installation)
* [Documents](#documents)
* [Features](#features)
* [Tech Stack](#tech-stack)

## Installation:
Recommended:

1. Download a pre-built release from [Github Releases](https://github.com/aaronriekenberg/rust-parallel/releases) for Linux or MacOS.
2. Extract the executable and put somewhere in your $PATH.

For manual installation/update:
1. [Install Rust](https://www.rust-lang.org/learn/get-started)
2. Install the latest version of this app from [crates.io](https://crates.io/crates/rust-parallel):
```
$ cargo install rust-parallel   
```
3. The same `cargo install rust-parallel` command will also update to the latest version after initial installation.

## Documents:
1. [Examples](https://github.com/aaronriekenberg/rust-parallel/wiki/Examples) - complete runnable commands to give an idea of overall features.
1. [Manual](https://github.com/aaronriekenberg/rust-parallel/wiki/Manual) - more detailed manual on how to use individual features.
1. [Benchmarks](https://github.com/aaronriekenberg/rust-parallel/wiki/Benchmarks)
1. [Output Interleaving](https://github.com/aaronriekenberg/rust-parallel/wiki/Output-Interleaving) - output interleaving in rust-parallel compared with other commands.

## Features:
* Use only safe rust.  
  * main.rs contains `#![forbid(unsafe_code)]`)
* Supports optional processing of inputs using regular expression capture groups.  This is implemented using the [`expand`](https://docs.rs/regex/latest/regex/struct.Captures.html#method.expand) function to replace specified capture groups with input data.
* Prevent [output interleaving](https://github.com/aaronriekenberg/rust-parallel/wiki/Output-Interleaving).
* Use only asynchronous operations supported by [tokio](https://tokio.rs), do not use any blocking operations.  This includes writing to stdout and stderr.
  * There is one exception to this: the `which` library used to build the path cache only has a blocking interface, so [`tokio::task::spawn_blocking`](https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html) is used to invoke this.
* Support arbitrarily large number of input lines, avoid `O(number of input lines)` memory usage.  In support of this:
  * [`tokio::sync::Semaphore`](https://docs.rs/tokio/latest/tokio/sync/struct.Semaphore.html) is used carefully to limit the number of commands that run concurrently.  Do not spawn tasks for all input lines immediately to limit memory usage.
* Cache resolved command paths so expensive lookup in $PATH is not done for every command executed.  This can be disabled with `--disable-path-cache` option.
* Support running commands on local machine only, not on remote machines.

## Tech Stack:
* [anyhow](https://github.com/dtolnay/anyhow) used for application error handling to propogate and format fatal errors.
* [clap](https://docs.rs/clap/latest/clap/) command line argument parser.
* [itertools](https://docs.rs/itertools/latest/itertools/) using [`multi_cartesian_product`](https://docs.rs/itertools/latest/itertools/trait.Itertools.html#method.multi_cartesian_product) to process `:::` command line inputs.
* [indicatif](https://github.com/console-rs/indicatif) optional graphical progress bar.
* [regex](https://github.com/rust-lang/regex) optional regular expression capture groups processing for `-r`/`--regex` option.
* [tokio](https://tokio.rs/) asynchronous runtime for rust.  From tokio this app uses:
  * `async` / `await` functions (aka coroutines)
  * Singleton `CommandLineArgs` instance using [`tokio::sync::OnceCell`](https://docs.rs/tokio/latest/tokio/sync/struct.OnceCell.html).
  * Asynchronous command execution using [`tokio::process::Command`](https://docs.rs/tokio/latest/tokio/process/struct.Command.html)
  * [`tokio::sync::Semaphore`](https://docs.rs/tokio/latest/tokio/sync/struct.Semaphore.html) used to limit number of commands that run concurrently.
  * [`tokio::sync::mpsc::channel`](https://docs.rs/tokio/latest/tokio/sync/mpsc/fn.channel.html) used to receive inputs from input task, and to send command outputs to an output writer task.  To await command completions, use the elegant property that when all `Senders` are dropped the channel is closed.
* [tracing](https://docs.rs/tracing/latest/tracing/) structured debug and warning logs.
  * [`tracing::Instrument`](https://docs.rs/tracing/latest/tracing/attr.instrument.html) is used to provide structured debug logs.
* [which](https://github.com/harryfei/which-rs) used to resolve command paths for path cache.
