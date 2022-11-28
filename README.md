# rust-parallel

Run commands in parallel, like a simple rust verision of [GNU Parallel](https://www.gnu.org/software/parallel/).

Just starting - more options to come :)

[![Crates.io][crates-badge]][crates-url]

[crates-badge]: https://img.shields.io/crates/v/rust-parallel.svg
[crates-url]: https://crates.io/crates/rust-parallel

Usage:
```
Run commands from stdin in parallel

Usage: rust-parallel [OPTIONS]

Options:
  -j, --jobs <JOBS>    Maximum number of commands to run in parallel, defauts to num cpus [default: 8]
  -s, --shell-enabled  Use /bin/sh shell to run commands, defaults to false
  -h, --help           Print help information
  -V, --version        Print version information
```

Little demo:

```
$ cargo build -v

$ cat test
echo hi
echo there
echo how
echo are
echo you

$ cat test | ./target/debug/rust-parallel -j5
are
hi
there
how
you
```

Using `awk` to form commands:

```
$ head -100 /usr/share/dict/words| awk '{printf "md5 -s %s\n", $1}' | ./target/debug/rust-parallel
MD5 ("Abba") = 5fa1e1f6e07a6fea3f2bb098e90a8de2
MD5 ("abaxial") = ac3a53971d52d9ce3277eadf03f13a5e
MD5 ("abaze") = 0b08c52aa63d947b6a5601ee975bc3a4
MD5 ("abaxile") = 21f5fc27d7d34117596e41d8c001087e
MD5 ("abbacomes") = 76640eb0c929bc97d016731bfbe9a4f8
MD5 ("abbacy") = 08aeac72800adc98d2aba540b6195921
MD5 ("Abbadide") = 7add1d6f008790fa6783bc8798d8c803
MD5 ("abb") = ea01e5fd8e4d8832825acdd20eac5104
```

With debug logs enabled:

```
$ cat test | RUST_LOG=debug ./target/debug/rust-parallel
2022-11-28T01:41:17.503219Z DEBUG rust_parallel: begin main
2022-11-28T01:41:17.503934Z DEBUG rust_parallel: command_line_args = CommandLineArgs { jobs: 12, shell_enabled: false }
2022-11-28T01:41:17.504141Z DEBUG rust_parallel: read line /bin/echo hi
2022-11-28T01:41:17.504196Z DEBUG rust_parallel: read line /bin/echo there
2022-11-28T01:41:17.504221Z DEBUG rust_parallel: read line /bin/echo how
2022-11-28T01:41:17.504243Z DEBUG rust_parallel: read line /bin/echo are
2022-11-28T01:41:17.504266Z DEBUG rust_parallel: read line /bin/echo you
2022-11-28T01:41:17.504349Z DEBUG rust_parallel: before wait_group.wait
2022-11-28T01:41:17.508247Z DEBUG rust_parallel: got command status = exit status: 0
there
2022-11-28T01:41:17.509790Z DEBUG rust_parallel: got command status = exit status: 0
hi
2022-11-28T01:41:17.511909Z DEBUG rust_parallel: got command status = exit status: 0
how
2022-11-28T01:41:17.512856Z DEBUG rust_parallel: got command status = exit status: 0
you
2022-11-28T01:41:17.514443Z DEBUG rust_parallel: got command status = exit status: 0
are
2022-11-28T01:41:17.514546Z DEBUG rust_parallel: end main
```
