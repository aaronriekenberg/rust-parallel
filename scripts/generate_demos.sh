#!/bin/bash

RUST_PARALLEL="rust-parallel"
VERSION=$($RUST_PARALLEL -V | cut -f2 -d' ')

echo "# Demos (rust-parallel $VERSION)"

echo 'There are 2 major ways to use rust-parallel:
1. Run commands from arguments using `:::` syntax to separate argument groups similar to GNU parallel.
1. Reading commands from stdin and/or input files similar to xargs.

Demos of command line arguments mode are first as it is simpler to understand:
1. [Commands from arguments](#commands-from-arguments)
1. [Small demo of echo commands](#small-demo-of-echo-commands)
1. [Debug logging](#debug-logging)
1. [Specifying command and intial arguments on command line](#specifying-command-and-intial-arguments-on-command-line)
1. [Using awk to form complete commands](#using-awk-to-form-complete-commands)
1. [Using as part of a shell pipeline](#using-as-part-of-a-shell-pipeline)
1. [Working on a set of files from find command](#working-on-a-set-of-files-from-find-command)
1. [Reading multiple inputs](#reading-multiple-inputs)
1. [Calling a bash function](#calling-a-bash-function)
1. [Calling a bash function commands from arguments](#calling-a-bash-function-commands-from-arguments)
'

echo '### Commands from arguments.

The `:::` separator can be used to run the [Cartesian Product](https://en.wikipedia.org/wiki/Cartesian_product) of command line arguments.  This is similar to the `:::` behavior in GNU Parallel.
'

echo '```
$ rust-parallel echo ::: A B ::: C D ::: E F G'
$RUST_PARALLEL echo ::: A B ::: C D ::: E F G || exit 1

echo '
$ rust-parallel echo hello ::: larry curly moe'
$RUST_PARALLEL echo hello ::: larry curly moe || exit 1

echo '
# run gzip -k on all *.html files in current directory
$ rust-parallel gzip -k ::: *.html
```'

echo '### Small demo of echo commands.  

Using command line arguments mode we can run 5 echo commands.

With `-j5` all commands run in parallel, with `-j1` commands run sequentially.
'

echo '```
$ rust-parallel -j5 echo ::: hi there how are you'
$RUST_PARALLEL -j5 echo ::: hi there how are you || exit 1

echo '
$ rust-parallel -j1 echo ::: hi there how are you'
$RUST_PARALLEL -j1 echo ::: hi there how are you || exit 1

echo '```'

echo 'Exactly equivalent to above a file `test` is created with 5 echo commands and piped to stdin of `rust-parallel`.

One advantage of reading input from stdin or input files is it can process much larger amounts of inputs than command line arguments.  Also this mode can be used as part of a shell pipeline.
'

echo '```
$ cat >./test <<EOL
echo hi
echo there
echo how
echo are
echo you
EOL'
cat >./test<<EOL
echo hi
echo there
echo how
echo are
echo you
EOL
if [ $? -ne 0 ]; then
  exit 1
fi

echo '
$ cat test | rust-parallel -j5'
cat test | $RUST_PARALLEL -j5 || exit 1

echo '
$ cat test | rust-parallel -j1'
cat test | $RUST_PARALLEL -j1 || exit 1

rm -f test

echo '```'

echo '### Debug logging.

Set environment variable `RUST_LOG=debug` to see debug output.

This logs structured information about command line arguments and commands being run.

Recommend enabling debug logging for all demos to understand what is happening in more detail.
'

echo '```
$ RUST_LOG=debug rust-parallel echo ::: hi there how are you | grep command_line_args | head -1'
RUST_LOG=debug $RUST_PARALLEL echo ::: hi there how are you | grep command_line_args | head -1 | ansi-stripper || exit 1

echo '
$ RUST_LOG=debug rust-parallel echo ::: hi there how are you | grep 'command_line_args:1''
RUST_LOG=debug $RUST_PARALLEL echo ::: hi there how are you | grep 'command_line_args:1' | ansi-stripper || exit 1
echo '```'

echo '### Specifying command and intial arguments on command line:

Here `md5 -s` will be prepended to each input line to form a command like `md5 -s aal`
'

echo '```
$ head -100 /usr/share/dict/words | rust-parallel md5 -s | head -10'
head -100 /usr/share/dict/words | $RUST_PARALLEL md5 -s | head -10 || exit 1
echo '```
'

echo '### Using `awk` to form complete commands:'

echo '
```
$ head -100 /usr/share/dict/words | awk '{printf "md5 -s %s\n", $1}' | rust-parallel | head -10'
head -100 /usr/share/dict/words | awk '{printf "md5 -s %s\n", $1}' | $RUST_PARALLEL | head -10 || exit 1
echo '```
'
echo '### Using as part of a shell pipeline.  

stdout and stderr from each command run are copied to stdout/stderr of the rust-parallel process.
'

echo '```
$ head -100 /usr/share/dict/words | rust-parallel md5 -s | grep -i abba'
head -100 /usr/share/dict/words | $RUST_PARALLEL md5 -s | grep -i abba || exit 1
echo '```
'

echo '### Working on a set of files from `find` command.  

The `-0` option works nicely with `find -print0` to handle filenames with newline or whitespace characters:
'

rm -fr testdir

echo '```'
echo '$ mkdir testdir
'
mkdir testdir || exit 1

echo "$ touch 'testdir/a b' 'testdir/b c' 'testdir/c d'
"
touch 'testdir/a b' 'testdir/b c' 'testdir/c d' || exit 1

echo '$ find testdir -type f -print0 | rust-parallel -0 gzip -f -k
'
find testdir -type f -print0 | $RUST_PARALLEL -0 gzip -f -k || exit 1

echo '$ ls -l testdir
'
ls -l testdir || exit 1

echo '```
'

rm -fr testdir

echo '### Reading multiple inputs.

By default `rust-parallel` reads input from stdin only.  The `-i` option can be used 1 or more times to override this behavior.  `-i -` means read from stdin, `-i ./test` means read from the file `./test`:
'

echo '```'
echo '$ cat >./test <<EOL
foo
bar
baz
EOL'
cat >./test <<EOL
foo
bar
baz
EOL
if [ $? -ne 0 ]; then
  exit 1
fi

echo '
$ head -5 /usr/share/dict/words | rust-parallel -i - -i ./test echo'
head -5 /usr/share/dict/words | $RUST_PARALLEL -i - -i ./test echo || exit 1

rm -f test

echo '```'

echo '### Calling a bash function.

Use `-s` shell mode so that each input line is passed to `/bin/bash -c` as a single argument:
'

echo '```'

echo '$ doit() {
  echo Doing it for $1
  sleep .5
  echo Done with $1
}'
doit() {
  echo Doing it for $1
  sleep .5
  echo Done with $1
}

echo '
$ export -f doit'
export -f doit

echo '
$ cat >./test <<EOL
doit 1
doit 2
doit 3
EOL'
cat >./test <<EOL
doit 1
doit 2
doit 3
EOL
if [ $? -ne 0 ]; then
  exit 1
fi

echo '
$ cat test | rust-parallel -s'

cat test | $RUST_PARALLEL -s || exit 1
rm -f test

echo '```
'

echo '### Calling a bash function commands from arguments.

Commands from arguments can also be used to invoke a bash function:
'

echo '```'

echo '$ logargs() {
  echo "logargs got $@"
}'
logargs() {
  echo "logargs got $@"
}

echo '
$ export -f logargs'
export -f logargs

echo '
$ rust-parallel -s logargs ::: A B C ::: D E F'
$RUST_PARALLEL -s logargs ::: A B C ::: D E F || exit 1

echo '```'