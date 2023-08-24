# rust-parallel

Command-line utility to execute commands in parallel and aggregate their output.

Similar interface to [GNU Parallel](https://www.gnu.org/software/parallel/parallel_examples.html) or [xargs](https://man7.org/linux/man-pages/man1/xargs.1.html) but implemented in rust and [tokio](https://tokio.rs) with handy features:
* Run commands from stdin, input files, or `:::` arguments
* Automatic parallelism to all cpus, can configure manually
* Transform inputs with regular expression [named or numbered capture groups](https://docs.rs/regex/latest/regex/#grouping-and-flags)
* Prevents [output interleaving](https://github.com/aaronriekenberg/rust-parallel/wiki/Output-Interleaving)
* [Very fast in benchmarks](https://github.com/aaronriekenberg/rust-parallel/wiki/Benchmarks)
* Shell mode to run bash functions or other shell commands
* TUI progress bar using [indicatif](https://github.com/console-rs/indicatif)
* Path cache
* Command timeouts
* Structured debug logging
 
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

## Tech Stack:
* [anyhow](https://github.com/dtolnay/anyhow) used for application error handling to propogate and format fatal errors.
* [clap](https://docs.rs/clap/latest/clap/) command line argument parser.
* [itertools](https://docs.rs/itertools/latest/itertools/) using [`multi_cartesian_product`](https://docs.rs/itertools/latest/itertools/trait.Itertools.html#method.multi_cartesian_product) to process `:::` command line inputs.
* [indicatif](https://github.com/console-rs/indicatif) optional graphical progress bar.
* [regex](https://github.com/rust-lang/regex) optional regular expression capture groups processing for `-r`/`--regex` option.
  * The [expand function](https://docs.rs/regex/latest/regex/struct.Captures.html#method.expand) is used to replace capture groups in command arguments with input data.
* [tokio](https://tokio.rs/) asynchronous runtime for rust.  From tokio this app uses:
  * `async` / `await` functions (aka coroutines)
  * Singleton `CommandLineArgs` instance using [`tokio::sync::OnceCell`](https://docs.rs/tokio/latest/tokio/sync/struct.OnceCell.html).
  * Asynchronous command execution using [`tokio::process::Command`](https://docs.rs/tokio/latest/tokio/process/struct.Command.html)
  * [`tokio::sync::Semaphore`](https://docs.rs/tokio/latest/tokio/sync/struct.Semaphore.html) used to limit number of commands that run concurrently.
  * [`tokio::sync::mpsc::channel`](https://docs.rs/tokio/latest/tokio/sync/mpsc/fn.channel.html) used to receive inputs from input task, and to send command outputs to an output writer task.  To await command completions, use the elegant property that when all `Senders` are dropped the channel is closed.
* [tracing](https://docs.rs/tracing/latest/tracing/) structured debug and warning logs.
  * [`tracing::Instrument`](https://docs.rs/tracing/latest/tracing/attr.instrument.html) is used to provide structured debug logs.
* [which](https://github.com/harryfei/which-rs) used to resolve command paths for path cache.
