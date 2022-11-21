# rust-parallel

Run commands in parallel, like a simple rust verision of [GNU Parallel](https://www.gnu.org/software/parallel/).

Just starting - more options to come :)

Little demo:

```
$ cargo build -v

$ cat test
echo hi
echo there
echo how
echo are
echo you

$ cat test | ./target/debug/rust-parallel

2022-11-21T23:03:35.372708Z  INFO rust_parallel: got command status = exit status: 0
2022-11-21T23:03:35.372752Z  INFO rust_parallel: got command stdout:
you

2022-11-21T23:03:35.372835Z  INFO rust_parallel: got command status = exit status: 0
2022-11-21T23:03:35.372860Z  INFO rust_parallel: got command stdout:
hi

2022-11-21T23:03:35.375877Z  INFO rust_parallel: got command status = exit status: 0
2022-11-21T23:03:35.375899Z  INFO rust_parallel: got command stdout:
there

2022-11-21T23:03:35.377244Z  INFO rust_parallel: got command status = exit status: 0
2022-11-21T23:03:35.377259Z  INFO rust_parallel: got command stdout:
how

2022-11-21T23:03:35.378694Z  INFO rust_parallel: got command status = exit status: 0
2022-11-21T23:03:35.378707Z  INFO rust_parallel: got command stdout:
are

```
