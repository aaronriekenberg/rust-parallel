# rust-parallel

Run commands in parallel, like a simple rust verision of [GNU Parallel](https://www.gnu.org/software/parallel/).

Just starting - more options to come :)

Usage:
```
Run commands from stdin in parallel

Usage: rust-parallel [OPTIONS]

Options:
  -j, --jobs <JOBS>  Maximum number of commands to run in parallel [default: 4]
  -h, --help         Print help information
  -V, --version      Print version information
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

With debug logs enabled:
```
cat test | RUST_LOG=debug ./target/debug/rust-parallel
2022-11-26T15:22:30.498186Z DEBUG rust_parallel: begin main
2022-11-26T15:22:30.498812Z DEBUG rust_parallel: command_line_args = CommandLineArgs { jobs: 4 }
2022-11-26T15:22:30.499004Z DEBUG rust_parallel: read line /bin/echo hi
2022-11-26T15:22:30.499060Z DEBUG rust_parallel: read line /bin/echo there
2022-11-26T15:22:30.499084Z DEBUG rust_parallel: read line /bin/echo how
2022-11-26T15:22:30.499115Z DEBUG rust_parallel: read line /bin/echo are
2022-11-26T15:22:30.499136Z DEBUG rust_parallel: read line /bin/echo you
2022-11-26T15:22:30.499220Z DEBUG rust_parallel: after loop join_set.len() = 5
2022-11-26T15:22:30.515913Z DEBUG rust_parallel: got command status = exit status: 0
hi
2022-11-26T15:22:30.516015Z DEBUG rust_parallel: join_next result = Ok(CommandInfo { _line_number: 1, command: "/bin/echo hi" })
2022-11-26T15:22:30.518279Z DEBUG rust_parallel: got command status = exit status: 0
are
2022-11-26T15:22:30.518388Z DEBUG rust_parallel: join_next result = Ok(CommandInfo { _line_number: 4, command: "/bin/echo are" })
2022-11-26T15:22:30.520035Z DEBUG rust_parallel: got command status = exit status: 0
there
2022-11-26T15:22:30.520164Z DEBUG rust_parallel: join_next result = Ok(CommandInfo { _line_number: 2, command: "/bin/echo there" })
2022-11-26T15:22:30.523486Z DEBUG rust_parallel: got command status = exit status: 0
how
2022-11-26T15:22:30.523586Z DEBUG rust_parallel: join_next result = Ok(CommandInfo { _line_number: 3, command: "/bin/echo how" })
2022-11-26T15:22:30.530829Z DEBUG rust_parallel: got command status = exit status: 0
you
2022-11-26T15:22:30.530912Z DEBUG rust_parallel: join_next result = Ok(CommandInfo { _line_number: 5, command: "/bin/echo you" })
2022-11-26T15:22:30.530942Z DEBUG rust_parallel: end main
```
