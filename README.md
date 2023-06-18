# rust-parallel

Command-line utility to execute commands in parallel and aggregate their output.

Similar interface to [GNU Parallel](https://www.gnu.org/software/parallel/parallel_examples.html) or [xargs](https://man7.org/linux/man-pages/man1/xargs.1.html) but implemented in rust and [tokio](https://tokio.rs).
* Supports running commands read from stdin or input files similar to xargs.
* Supports `:::` syntax to run all combinations of argument groups similar to GNU Parallel.

Prevents [output interleaving](https://github.com/aaronriekenberg/rust-parallel/wiki/Output-Interleaving) and is [very fast](https://github.com/aaronriekenberg/rust-parallel/wiki/Benchmarks).

See the [demos](#demos) for example usage.

[crates-badge]: https://img.shields.io/crates/v/rust-parallel.svg
[crates-url]: https://crates.io/crates/rust-parallel

[ci-badge]: https://github.com/aaronriekenberg/rust-parallel/actions/workflows/CI.yml/badge.svg
[ci-url]: https://github.com/aaronriekenberg/rust-parallel/actions/workflows/CI.yml 

[![Crates.io][crates-badge]][crates-url] [![CI workflow][ci-badge]][ci-url]

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
          Optional command and initial arguments.

          If this contains 1 or more ::: delimiters the cartesian product of arguments from all groups are run.

Options:
  -d, --discard-output <DISCARD_OUTPUT>
          Discard output for commands

          Possible values:
          - stdout: Redirect stdout for commands to /dev/null
          - stderr: Redirect stderr for commands to /dev/null
          - all:    Redirect stdout and stderr for commands to /dev/null

  -i, --input-file <INPUT_FILE>
          Input file or - for stdin.  Defaults to stdin if no inputs are specified

  -j, --jobs <JOBS>
          Maximum number of commands to run in parallel, defauts to num cpus

          [default: 8]

  -0, --null-separator
          Use null separator for reading input files instead of newline

  -s, --shell
          Use shell mode for running commands.

          Each command line is passed to "<shell-path> -c" as a single argument.

      --channel-capacity <CHANNEL_CAPACITY>
          Input and output channel capacity, defaults to num cpus * 2

          [default: 16]

      --shell-path <SHELL_PATH>
          Path to shell to use for shell mode

          [default: /bin/bash]

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version
```

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

## Demos:
There are 2 major ways to use rust-parallel:
1. Command line arguments mode using `:::` syntax to separate argument groups similar to GNU parallel.
1. Reading commands from stdin and/or input files similar to xargs.

Demos of command line arguments mode are first as it is simpler to understand:
1. [Commands from arguments mode](#commands-from-arguments-mode)
1. [Commands from arguments mode bash function](#commands-from-arguments-mode-bash-function)
1. [Small demo of 5 echo commands](#small-demo-of-5-echo-commands)
1. [Debug logging](#debug-logging)
1. [Specifying command and intial arguments on command line](#specifying-command-and-intial-arguments-on-command-line)
1. [Using awk to form complete commands](#using-awk-to-form-complete-commands)
1. [Using as part of a shell pipeline](#using-as-part-of-a-shell-pipeline)
1. [Working on a set of files from find command](#working-on-a-set-of-files-from-find-command)
1. [Reading multiple inputs](#reading-multiple-inputs)
1. [Calling a bash function](#calling-a-bash-function)

### Commands from arguments mode.

When `-c/--commands-from-args` is specified, the `:::` separator can be used to run the [Cartesian Product](https://en.wikipedia.org/wiki/Cartesian_product) of command line arguments.  This is similar to the `:::` behavior in GNU Parallel.

```
$ rust-parallel -c echo ::: A B ::: C D ::: E F G
B C F
A D E
A C G
A D F
A D G
A C F
B C E
A C E
B D F
B D E
B D G
B C G

$ rust-parallel -c echo hello ::: larry curly moe
hello curly
hello larry
hello moe

# run gzip -k on all *.html files in current directory
$ rust-parallel -c gzip -k ::: *.html
```

### Commands from arguments mode bash function.

Commands from arguments mode can be used to invoke a bash function.

```
$ logargs() {
  echo "logargs got $@"
}

$ export -f logargs

$ rust-parallel -c -s logargs ::: A B C ::: D E F
logargs got A F
logargs got A D
logargs got B E
logargs got C E
logargs got B D
logargs got B F
logargs got A E
logargs got C D
logargs got C F
```

### Small demo of 5 echo commands.  

Using command line arguments mode we can run 5 echo commands.

With `-j5` all commands run in parallel, with `-j1` commands run sequentially.

```
$ rust-parallel -j5 -c echo ::: hi there how are you
how
there
you
are
hi

$ rust-parallel -j1 -c echo ::: hi there how are you
hi
there
how
are
you
```

Exactly equivalent to above a file `test` is created with 5 echo commands and piped to stdin of `rust-parallel`.

One advantage of reading input from stdin or input files is it can process much larger amounts of inputs than command line arguments.  Also this mode can be used as part of a shell pipeline.

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

### Debug logging.

Set environment variable `RUST_LOG=debug` to see debug output.

This logs structured information about command line arguments and commands being run.

Recommend enabling debug logging for all demos to understand what is happening in more detail.

```
$ RUST_LOG=debug rust-parallel -c echo ::: hi there how are you | grep command_line_args

2023-06-16T12:14:45.602832Z DEBUG try_main: rust_parallel::command_line_args: command_line_args = CommandLineArgs { commands_from_args: true, discard_output: None, input_file: [], jobs: 8, null_separator: false, shell: false, channel_capacity: 16, shell_path: "/bin/bash", command_and_initial_arguments: ["echo", ":::", "hi", "there", "how", "are", "you"] }

$ RUST_LOG=debug rust-parallel -c echo ::: hi there how are you | grep 'command_line_args:1'

2023-06-16T12:15:18.408524Z DEBUG Command::run{cmd_args=["echo", "there"] line=command_line_args:1}: rust_parallel::command: begin run
2023-06-16T12:15:18.410259Z DEBUG Command::run{cmd_args=["echo", "there"] line=command_line_args:1 child_pid=12523}: rust_parallel::command: spawned child process, awaiting output
2023-06-16T12:15:18.413080Z DEBUG Command::run{cmd_args=["echo", "there"] line=command_line_args:1 child_pid=12523}: rust_parallel::command: command exit status = exit status: 0
2023-06-16T12:15:18.413125Z DEBUG Command::run{cmd_args=["echo", "there"] line=command_line_args:1 child_pid=12523}: rust_parallel::command: end run
```

### Specifying command and intial arguments on command line:

Here `md5 -s` will be prepended to each input line to form a command like `md5 -s aal`

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

### Using `awk` to form complete commands:

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

### Using as part of a shell pipeline.  

stdout and stderr from each command run are copied to stdout/stderr of the rust-parallel process.

```
$ head -100 /usr/share/dict/words | rust-parallel md5 -s | grep -i abba
MD5 ("Abba") = 5fa1e1f6e07a6fea3f2bb098e90a8de2
MD5 ("abbacomes") = 76640eb0c929bc97d016731bfbe9a4f8
MD5 ("abbacy") = 08aeac72800adc98d2aba540b6195921
MD5 ("Abbadide") = 7add1d6f008790fa6783bc8798d8c803
```

### Working on a set of files from `find` command.  

The `-0` option works nicely with `find -print0` to handle filenames with newline or whitespace characters:

```
$ mkdir testdir

$ touch 'testdir/a b' 'testdir/b c' 'testdir/c d'

$ find testdir -type f -print0 | rust-parallel -0 gzip -f -k

$ ls testdir
'a b'  'a b.gz'  'b c'  'b c.gz'  'c d'  'c d.gz'
```


### Reading multiple inputs.

By default `rust-parallel` reads input from stdin only.  The `-i` option can be used 1 or more times to override this behavior.  `-i -` means read from stdin, `-i ./test` means read from the file `./test`:

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

### Calling a bash function.

Use `-s` shell mode so that each input line is passed to `/bin/bash -c` as a single argument:

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


## Benchmarks:
See the [wiki page for benchmarks](https://github.com/aaronriekenberg/rust-parallel/wiki/Benchmarks).

## Features:
* Use only safe rust.  
  * main.rs contains `#![forbid(unsafe_code)]`)
* Prevent [output interleaving](https://github.com/aaronriekenberg/rust-parallel/wiki/Output-Interleaving).
* Use only asynchronous operations supported by [tokio](https://tokio.rs), do not use any blocking operations.  This includes writing to stdout and stderr.
* Support arbitrarily large number of input lines, avoid `O(number of input lines)` memory usage.  In support of this:
  * [`tokio::sync::Semaphore`](https://docs.rs/tokio/latest/tokio/sync/struct.Semaphore.html) is used carefully to limit the number of commands that run concurrently.  Do not spawn tasks for all input lines immediately to limit memory usage.
* Support running commands on local machine only, not on remote machines.

## Tech Stack:
* [anyhow](https://github.com/dtolnay/anyhow) used for application error handling to propogate and format fatal errors.
* [clap](https://docs.rs/clap/latest/clap/) command line argument parser.
* [itertools](https://docs.rs/itertools/latest/itertools/) using [`multi_cartesian_product`](https://docs.rs/itertools/latest/itertools/trait.Itertools.html#method.multi_cartesian_product) to process `:::` command line inputs.
* [tokio](https://tokio.rs/) asynchronous runtime for rust.  From tokio this app uses:
  * `async` / `await` functions (aka coroutines)
  * Singleton `CommandLineArgs` instance using [`tokio::sync::OnceCell`](https://docs.rs/tokio/latest/tokio/sync/struct.OnceCell.html).
  * Asynchronous command execution using [`tokio::process::Command`](https://docs.rs/tokio/latest/tokio/process/struct.Command.html)
  * [`tokio::sync::Semaphore`](https://docs.rs/tokio/latest/tokio/sync/struct.Semaphore.html) used to limit number of commands that run concurrently.
  * [`tokio::sync::mpsc::channel`](https://docs.rs/tokio/latest/tokio/sync/mpsc/fn.channel.html) used to receive inputs from input task, and to send command outputs to an output writer task.  To await command completions, use the elegant property that when all `Senders` are dropped the channel is closed.
* [tracing](https://docs.rs/tracing/latest/tracing/) structured debug and warning logs.
  * [`tracing::Instrument`](https://docs.rs/tracing/latest/tracing/attr.instrument.html) is used to provide structured debug logs.
