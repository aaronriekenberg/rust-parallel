# rust-parallel

Command-line utility to execute commands in parallel and aggregate their output.

Similar interface to [GNU Parallel](https://www.gnu.org/software/parallel/parallel_examples.html) or [xargs](https://man.openbsd.org/xargs) but implemented in rust and [tokio](https://tokio.rs).

Always executes 1 process for each input line, similar to xargs `-n1` or `-L1` options.

Being written in asynchronous rust it is quite fast - see [benchmarks](https://github.com/aaronriekenberg/rust-parallel/wiki/Benchmarks).

[![Crates.io][crates-badge]][crates-url]

[crates-badge]: https://img.shields.io/crates/v/rust-parallel.svg
[crates-url]: https://crates.io/crates/rust-parallel

## Contents:
* [Usage](#usage)
* [Installation](#installation)
* [Demos](#demos)
* [Benchmarks](#benchmarks)
* [Features](#features)
* [Tech Stack](#tech-stack)

## Usage:
```
$ rust-parallel --help
Execute commands in parallel

By Aaron Riekenberg <aaron.riekenberg@gmail.com>

https://github.com/aaronriekenberg/rust-parallel
https://crates.io/crates/rust-parallel

Usage: rust-parallel [OPTIONS] [COMMAND_AND_INITIAL_ARGUMENTS]...

Arguments:
  [COMMAND_AND_INITIAL_ARGUMENTS]...
          Optional command and initial arguments to run for each input line

Options:
  -i, --input <INPUT>
          Input file or - for stdin.  Defaults to stdin if no inputs are specified

  -j, --jobs <JOBS>
          Maximum number of commands to run in parallel, defauts to num cpus

          [default: 8]

  -0, --null-separator
          Use null separator for reading input instead of newline

  -s, --shell
          Use shell for running commands.

          If $SHELL is set use it else use /bin/sh.

          Each input line is passed to $SHELL -c <line> as a single argument.

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version
```

## Installation:
Recommended:

1. Download a pre-built release from [Github Releases](https://github.com/aaronriekenberg/rust-parallel/releases).
2. Extract the executable and put somewhere in your $PATH.

For manual installation/update:
1. [Install Rust](https://www.rust-lang.org/learn/get-started)
2. Install the latest version of this app from [crates.io](https://crates.io/crates/rust-parallel):
```
$ cargo install rust-parallel   
```
3. The same `cargo install rust-parallel` command will also update to the latest version after initial installation.

## Demos:

1. Small demo of 5 echo commands.  With `-j5` all 5 commands are run in parallel.  With `-j1` commands are run sequentially:

```
$ cat >./test <<EOL
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

$ cat test | rust-parallel -j1
hi
there
how
are
you
```

2. Specifying command and intial arguments on command line:

```
$ head -100 /usr/share/dict/words | rust-parallel md5 -s
MD5 ("aal") = ff45e881572ca2c987460932660d320c
MD5 ("A") = 7fc56270e7a70fa81a5935b72eacbe29
MD5 ("aardvark") = 88571e5d5e13a4a60f82cea7802f6255
MD5 ("aalii") = 0a1ea2a8d75d02ae052f8222e36927a5
MD5 ("aam") = 35c2d90f7c06b623fe763d0a4e5b7ed9
MD5 ("aa") = 4124bc0a9335c27f086f24ba207a4912
MD5 ("a") = 0cc175b9c0f1b6a831c399e269772661
MD5 ("Aani") = e9b22dd6213c3d29648e8ad7a8642f2f
MD5 ("Aaron") = 1c0a11cc4ddc0dbd3fa4d77232a4e22e
MD5 ("aardwolf") = 66a4a1a2b442e8d218e8e99100069877
```

3. Using `awk` to form complete commands:

```
$ head -100 /usr/share/dict/words | awk '{printf "md5 -s %s\n", $1}' | rust-parallel
MD5 ("Abba") = 5fa1e1f6e07a6fea3f2bb098e90a8de2
MD5 ("abaxial") = ac3a53971d52d9ce3277eadf03f13a5e
MD5 ("abaze") = 0b08c52aa63d947b6a5601ee975bc3a4
MD5 ("abaxile") = 21f5fc27d7d34117596e41d8c001087e
MD5 ("abbacomes") = 76640eb0c929bc97d016731bfbe9a4f8
MD5 ("abbacy") = 08aeac72800adc98d2aba540b6195921
MD5 ("Abbadide") = 7add1d6f008790fa6783bc8798d8c803
MD5 ("abb") = ea01e5fd8e4d8832825acdd20eac5104
```

4. Using as part of a shell pipeline.  stdout and stderr from each command run are copied to stdout/stderr of the rust-parallel process.

```
$ head -100 /usr/share/dict/words | rust-parallel md5 -s | grep -i abba
MD5 ("Abba") = 5fa1e1f6e07a6fea3f2bb098e90a8de2
MD5 ("abbacomes") = 76640eb0c929bc97d016731bfbe9a4f8
MD5 ("abbacy") = 08aeac72800adc98d2aba540b6195921
MD5 ("Abbadide") = 7add1d6f008790fa6783bc8798d8c803
```

5. Working on a set of files from `find` command.  The `-0` option works nicely with `find -print0` to handle filenames with newline or whitespace characters:

```
$ mkdir testdir

$ touch 'testdir/a b' 'testdir/b c' 'testdir/c d'

$ find testdir -type f -print0 | rust-parallel -0 gzip -f -k

$ ls testdir
'a b'  'a b.gz'  'b c'  'b c.gz'  'c d'  'c d.gz'
```


6. By default `rust-parallel` reads input from stdin only.  The `-i` option can be used 1 or more times to override this behavior.  `-i -` means read from stdin, `-i ./test` means read from the file `./test`:

```
$ cat >./test <<EOL
foo
bar
baz
EOL

$ head -5 /usr/share/dict/words | rust-parallel -i - -i ./test echo
A
aalii
aa
a
aal
bar
foo
baz
```

7. Calling a bash function, use -s shell mode so that each line is passed to `/bin/bash -c` as a single argument:

```
$ doit() {
  echo Doing it for $1
  sleep 2
  echo Done with $1
}

$ export -f doit

$ cat >./test <<EOL
doit 1
doit 2
doit 3
EOL

$ cat test | rust-parallel -s
Doing it for 1
Done with 1
Doing it for 3
Done with 3
Doing it for 2
Done with 2

```

8. Set environment variable `RUST_LOG=debug` to see debug output.

```
$ head -10 /usr/share/dict/words | RUST_LOG=debug rust-parallel md5 -s
```

## Benchmarks:
See the [wiki page for benchmarks](https://github.com/aaronriekenberg/rust-parallel/wiki/Benchmarks).

## Features:
* Use only safe rust.
* Use only asynchronous operations supported by [tokio](https://tokio.rs), do not use any blocking operations.  This includes writing to stdout and stderr.
* Support arbitrarily large number of input lines, avoid `O(number of input lines)` memory usage.  In support of this:
  * [`tokio::sync::Semaphore`](https://docs.rs/tokio/latest/tokio/sync/struct.Semaphore.html) is used carefully to limit the number of commands that run concurrently.  Do not spawn tasks for all input lines immediately to limit memory usage.
* Support running commands on local machine only, not on remote machines.

## Tech Stack:
* [anyhow](https://github.com/dtolnay/anyhow) used for application error handling to propogate and format fatal errors.
* [clap](https://docs.rs/clap/latest/clap/) command line argument parser.
* [tokio](https://tokio.rs/) asynchronous runtime for rust.  From tokio this app uses:
  * `async` / `await` functions (aka coroutines)
  * Singleton `CommandLineArgs` instance using [`tokio::sync::OnceCell`](https://docs.rs/tokio/latest/tokio/sync/struct.OnceCell.html).
  * Asynchronous command execution using [`tokio::process::Command`](https://docs.rs/tokio/latest/tokio/process/struct.Command.html)
  * [`tokio::sync::Semaphore`](https://docs.rs/tokio/latest/tokio/sync/struct.Semaphore.html) used to limit number of commands that run concurrently.
     * Life would be a bit easier if `acquire_many` took a `usize` parameter: https://github.com/tokio-rs/tokio/issues/4446
  * [`tokio::sync::Mutex`](https://docs.rs/tokio/latest/tokio/sync/struct.Mutex.html) used to protect access to stdout/stderr to prevent interleaved command output.
* [tracing](https://docs.rs/tracing/latest/tracing/) structured debug and warning logs.
  * [`tracing::Instrument`](https://docs.rs/tracing/latest/tracing/attr.instrument.html) is used to provide structured debug logs on methods in `command::Command` and `command::CommandService`.
