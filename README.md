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
2022-11-26T15:47:41.898980Z DEBUG rust_parallel: begin main
2022-11-26T15:47:41.899717Z DEBUG rust_parallel: command_line_args = CommandLineArgs { jobs: 4 }
2022-11-26T15:47:41.899933Z DEBUG rust_parallel: read line echo hi
2022-11-26T15:47:41.900001Z DEBUG rust_parallel: read line echo there
2022-11-26T15:47:41.900049Z DEBUG rust_parallel: read line echo how
2022-11-26T15:47:41.900140Z DEBUG rust_parallel: read line echo are
2022-11-26T15:47:41.900227Z DEBUG rust_parallel: read line echo you
2022-11-26T15:47:41.900391Z DEBUG rust_parallel: after spawn_commands join_set.len() = 5
2022-11-26T15:47:41.906264Z DEBUG rust_parallel: got command status = exit status: 0
hi
2022-11-26T15:47:41.906433Z DEBUG rust_parallel: got command status = exit status: 0
how
2022-11-26T15:47:41.906371Z DEBUG rust_parallel: join_next result = Ok(CommandInfo { _line_number: 1, command: "echo hi" })
2022-11-26T15:47:41.906479Z DEBUG rust_parallel: got command status = exit status: 0
there
2022-11-26T15:47:41.906519Z DEBUG rust_parallel: join_next result = Ok(CommandInfo { _line_number: 3, command: "echo how" })
2022-11-26T15:47:41.906556Z DEBUG rust_parallel: join_next result = Ok(CommandInfo { _line_number: 2, command: "echo there" })
2022-11-26T15:47:41.906570Z DEBUG rust_parallel: got command status = exit status: 0
are
2022-11-26T15:47:41.906638Z DEBUG rust_parallel: join_next result = Ok(CommandInfo { _line_number: 4, command: "echo are" })
2022-11-26T15:47:41.911268Z DEBUG rust_parallel: got command status = exit status: 0
you
2022-11-26T15:47:41.911376Z DEBUG rust_parallel: join_next result = Ok(CommandInfo { _line_number: 5, command: "echo you" })
2022-11-26T15:47:41.911408Z DEBUG rust_parallel: end main
```
