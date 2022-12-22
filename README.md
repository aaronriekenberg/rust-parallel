# rust-parallel

Command-line utility to execute commands in parallel and aggregate their output.

Something like a simple rust version of [GNU Parallel](https://www.gnu.org/software/parallel/).

Being written in asynchronous rust it is quite fast - see [benchmarks](https://github.com/aaronriekenberg/rust-parallel/wiki/Benchmarks).

[![Crates.io][crates-badge]][crates-url]

[crates-badge]: https://img.shields.io/crates/v/rust-parallel.svg
[crates-url]: https://crates.io/crates/rust-parallel

# Goals:
* Use only safe rust.
* Use only asynchronous operations supported by [tokio](https://tokio.rs), do not use any blocking operations.
* Support arbitrarily large number of input lines, avoid `O(number of input lines)` memory usage.  In support of this:
  * [`tokio::sync::Semaphore`](https://docs.rs/tokio/latest/tokio/sync/struct.Semaphore.html) is used carefully to limit the number of commands that run concurrently.  Do not spawn tasks for all input lines immediately to limit memory usage.
  * [`awaitgroup::WaitGroup`](https://crates.io/crates/awaitgroup) is used to wait for all async functions to finish.  Internally this is a counter and uses a constant amount of memory.
* Support running commands on local machine only, not on remote machines.

# Tech Stack:
* [anyhow](https://github.com/dtolnay/anyhow) used for application error handling to propogate and format fatal errors.
* [awaitgroup](https://crates.io/crates/awaitgroup) used to await completion of all async functions.
* [clap](https://docs.rs/clap/latest/clap/) command line argument parser.
* [tokio](https://tokio.rs/) asynchronous runtime for rust.  From tokio this app uses:
  * `async` / `await` functions (aka coroutines)
  * Singleton `CommandLineArgs` instance using [`tokio::sync::OnceCell`](https://docs.rs/tokio/latest/tokio/sync/struct.OnceCell.html).
  * Asynchronous command execution using [`tokio::process::Command`](https://docs.rs/tokio/latest/tokio/process/struct.Command.html)
  * [`tokio::sync::Semaphore`](https://docs.rs/tokio/latest/tokio/sync/struct.Semaphore.html) used to limit number of commands that run concurrently.
  * [`tokio::sync::Mutex`](https://docs.rs/tokio/latest/tokio/sync/struct.Mutex.html) used to protect access to stdout/stderr to prevent interleaved command output.
* [tracing](https://docs.rs/tracing/latest/tracing/) used for debug and warning logs.

# Installation:
1. [Install Rust](https://www.rust-lang.org/learn/get-started)
2. Install the latest version of this app from [crates.io](https://crates.io/crates/rust-parallel):
```
$ cargo install rust-parallel   
```

# Usage:
```
$ rust-parallel -h

Run commands in parallel

Usage: rust-parallel [OPTIONS] [INPUTS]...

Arguments:
  [INPUTS]...  Input file or - for stdin.  Defaults to stdin if no inputs are specified

Options:
  -j, --jobs <JOBS>    Maximum number of commands to run in parallel, defauts to num cpus [default: 12]
  -s, --shell-enabled  Use /bin/sh -c shell to run commands
  -h, --help           Print help information
  -V, --version        Print version information
```

# Demos:

Small demo of 5 echo commands:

```
$ cat >./test <<EOL
# input can contain comment lines (starting with #) and blank lines too

echo hi
echo there
echo how
echo are
echo you
EOL

$ cat test | rust-parallel -j5
are
hi
there
how
you
```

Using `awk` to form commands:

```
$ head -100 /usr/share/dict/words| awk '{printf "md5 -s %s\n", $1}' | rust-parallel
MD5 ("Abba") = 5fa1e1f6e07a6fea3f2bb098e90a8de2
MD5 ("abaxial") = ac3a53971d52d9ce3277eadf03f13a5e
MD5 ("abaze") = 0b08c52aa63d947b6a5601ee975bc3a4
MD5 ("abaxile") = 21f5fc27d7d34117596e41d8c001087e
MD5 ("abbacomes") = 76640eb0c929bc97d016731bfbe9a4f8
MD5 ("abbacy") = 08aeac72800adc98d2aba540b6195921
MD5 ("Abbadide") = 7add1d6f008790fa6783bc8798d8c803
MD5 ("abb") = ea01e5fd8e4d8832825acdd20eac5104
```

Using as part of a shell pipeline.  stdout and stderr from each command run are copied to stdout/stderr of the rust-parallel process.

```
$ head -100 /usr/share/dict/words| awk '{printf "md5 -s %s\n", $1}' | rust-parallel | grep -i abba
MD5 ("Abba") = 5fa1e1f6e07a6fea3f2bb098e90a8de2
MD5 ("abbacomes") = 76640eb0c929bc97d016731bfbe9a4f8
MD5 ("abbacy") = 08aeac72800adc98d2aba540b6195921
MD5 ("Abbadide") = 7add1d6f008790fa6783bc8798d8c803
```

Using input file.  Multiple inputs can be specified, `-` means stdin:

```
$ cat >./test1 <<EOL
echo hi
echo there
echo how
EOL

$ cat >./test2 <<EOL
echo are
echo you
EOL

$ cat test2 | rust-parallel test1 -
there
how
hi
are
you

```

With debug logs enabled:

```
$ cat test | RUST_LOG=debug rust-parallel
2022-12-22T23:03:25.233196Z DEBUG rust_parallel: begin try_main
2022-12-22T23:03:25.233895Z DEBUG rust_parallel::command_line_args: command_line_args = CommandLineArgs { jobs: 12, shell_enabled: false, inputs: [] }
2022-12-22T23:03:25.233958Z DEBUG rust_parallel::command: begin run_commands
2022-12-22T23:03:25.233989Z DEBUG rust_parallel::command: begin process_one_input input = Stdin
2022-12-22T23:03:25.234169Z DEBUG rust_parallel::command: process_one_input_line # input can contain comment lines (starting with #) and blank lines too
2022-12-22T23:03:25.234197Z DEBUG rust_parallel::command: process_one_input_line
2022-12-22T23:03:25.234211Z DEBUG rust_parallel::command: process_one_input_line echo hi
2022-12-22T23:03:25.234258Z DEBUG rust_parallel::command: process_one_input_line echo there
2022-12-22T23:03:25.234289Z DEBUG rust_parallel::command: process_one_input_line echo how
2022-12-22T23:03:25.234312Z DEBUG rust_parallel::command: process_one_input_line echo are
2022-12-22T23:03:25.234339Z DEBUG rust_parallel::command: process_one_input_line echo you
2022-12-22T23:03:25.234356Z DEBUG rust_parallel::command: begin run command = Command { input_line_number: InputLineNumber { input: Stdin, line_number: 3 }, command: "echo hi", shell_enabled: false } worker = WaitGroup { count: 5 } permit = OwnedSemaphorePermit { sem: Semaphore { ll_sem: Semaphore { permits: 7 } }, permits: 1 }
2022-12-22T23:03:25.234369Z DEBUG rust_parallel::command: begin run command = Command { input_line_number: InputLineNumber { input: Stdin, line_number: 4 }, command: "echo there", shell_enabled: false } worker = WaitGroup { count: 5 } permit = OwnedSemaphorePermit { sem: Semaphore { ll_sem: Semaphore { permits: 7 } }, permits: 1 }
2022-12-22T23:03:25.234388Z DEBUG rust_parallel::command: begin run command = Command { input_line_number: InputLineNumber { input: Stdin, line_number: 5 }, command: "echo how", shell_enabled: false } worker = WaitGroup { count: 5 } permit = OwnedSemaphorePermit { sem: Semaphore { ll_sem: Semaphore { permits: 7 } }, permits: 1 }
2022-12-22T23:03:25.234420Z DEBUG rust_parallel::command: begin run command = Command { input_line_number: InputLineNumber { input: Stdin, line_number: 6 }, command: "echo are", shell_enabled: false } worker = WaitGroup { count: 5 } permit = OwnedSemaphorePermit { sem: Semaphore { ll_sem: Semaphore { permits: 7 } }, permits: 1 }
2022-12-22T23:03:25.234425Z DEBUG rust_parallel::command: end process_one_input input = Stdin
2022-12-22T23:03:25.234486Z DEBUG rust_parallel::command: before wait_group.wait wait_group = WaitGroup { count: 5 }
2022-12-22T23:03:25.234450Z DEBUG rust_parallel::command: begin run command = Command { input_line_number: InputLineNumber { input: Stdin, line_number: 7 }, command: "echo you", shell_enabled: false } worker = WaitGroup { count: 5 } permit = OwnedSemaphorePermit { sem: Semaphore { ll_sem: Semaphore { permits: 7 } }, permits: 1 }
2022-12-22T23:03:25.238679Z DEBUG rust_parallel::command: got command status = exit status: 0
there
2022-12-22T23:03:25.238955Z DEBUG rust_parallel::command: end run command = Command { input_line_number: InputLineNumber { input: Stdin, line_number: 4 }, command: "echo there", shell_enabled: false } worker = WaitGroup { count: 5 } permit = OwnedSemaphorePermit { sem: Semaphore { ll_sem: Semaphore { permits: 7 } }, permits: 1 }
2022-12-22T23:03:25.239721Z DEBUG rust_parallel::command: got command status = exit status: 0
hi
2022-12-22T23:03:25.239993Z DEBUG rust_parallel::command: end run command = Command { input_line_number: InputLineNumber { input: Stdin, line_number: 3 }, command: "echo hi", shell_enabled: false } worker = WaitGroup { count: 4 } permit = OwnedSemaphorePermit { sem: Semaphore { ll_sem: Semaphore { permits: 8 } }, permits: 1 }
2022-12-22T23:03:25.241048Z DEBUG rust_parallel::command: got command status = exit status: 0
are
2022-12-22T23:03:25.241316Z DEBUG rust_parallel::command: end run command = Command { input_line_number: InputLineNumber { input: Stdin, line_number: 6 }, command: "echo are", shell_enabled: false } worker = WaitGroup { count: 3 } permit = OwnedSemaphorePermit { sem: Semaphore { ll_sem: Semaphore { permits: 9 } }, permits: 1 }
2022-12-22T23:03:25.241877Z DEBUG rust_parallel::command: got command status = exit status: 0
how
2022-12-22T23:03:25.242076Z DEBUG rust_parallel::command: end run command = Command { input_line_number: InputLineNumber { input: Stdin, line_number: 5 }, command: "echo how", shell_enabled: false } worker = WaitGroup { count: 2 } permit = OwnedSemaphorePermit { sem: Semaphore { ll_sem: Semaphore { permits: 10 } }, permits: 1 }
2022-12-22T23:03:25.242614Z DEBUG rust_parallel::command: got command status = exit status: 0
you
2022-12-22T23:03:25.242776Z DEBUG rust_parallel::command: end run command = Command { input_line_number: InputLineNumber { input: Stdin, line_number: 7 }, command: "echo you", shell_enabled: false } worker = WaitGroup { count: 1 } permit = OwnedSemaphorePermit { sem: Semaphore { ll_sem: Semaphore { permits: 11 } }, permits: 1 }
2022-12-22T23:03:25.242829Z DEBUG rust_parallel::command: end run_commands
2022-12-22T23:03:25.242857Z DEBUG rust_parallel: end try_main
```
