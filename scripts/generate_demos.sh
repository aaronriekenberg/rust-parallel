#!/bin/bash -e

RUST_PARALLEL="./target/debug/rust-parallel"
VERSION=$($RUST_PARALLEL -V | cut -f2 -d' ')

echo "## Demos for rust-parallel $VERSION"

echo 'There are 2 major ways to use rust-parallel:
1. Run commands from arguments using `:::` syntax to separate argument groups similar to GNU parallel.
1. Reading commands from stdin and/or input files similar to xargs.

Demos of command from arguments are first as it is simpler to understand:
1. [Commands from arguments](#commands-from-arguments)
1. [Small demo of echo commands](#small-demo-of-echo-commands)
1. [Debug logging](#debug-logging)
1. [Timeout](#timeout)
1. [Progress bar](#progress-bar)
1. [Specifying command and initial arguments on command line](#specifying-command-and-initial-arguments-on-command-line)
1. [Using awk to form complete commands](#using-awk-to-form-complete-commands)
1. [Using as part of a shell pipeline](#using-as-part-of-a-shell-pipeline)
1. [Working on a set of files from find command](#working-on-a-set-of-files-from-find-command)
1. [Reading multiple inputs](#reading-multiple-inputs)
1. [Bash Function](#bash-function)
'

echo '## Commands from arguments.

The `:::` separator can be used to run the [Cartesian Product](https://en.wikipedia.org/wiki/Cartesian_product) of command line arguments.  This is similar to the `:::` behavior in GNU Parallel.
'

echo '```
$ rust-parallel echo ::: A B ::: C D ::: E F G'
$RUST_PARALLEL echo ::: A B ::: C D ::: E F G

echo '
$ rust-parallel echo hello ::: larry curly moe'
$RUST_PARALLEL echo hello ::: larry curly moe

echo '
# run gzip -k on all *.html files in current directory
$ rust-parallel gzip -k ::: *.html
```'

echo '
## Small demo of echo commands.

Using command line arguments we can run 5 echo commands.

With `-j5` all commands run in parallel, with `-j1` commands run sequentially.
'

echo '```
$ rust-parallel -j5 echo ::: hi there how are you'
$RUST_PARALLEL -j5 echo ::: hi there how are you

echo '
$ rust-parallel -j1 echo ::: hi there how are you'
$RUST_PARALLEL -j1 echo ::: hi there how are you

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

echo '
$ cat test | rust-parallel -j5'
cat test | $RUST_PARALLEL -j5

echo '
$ cat test | rust-parallel -j1'
cat test | $RUST_PARALLEL -j1

rm -f test

echo '```'

echo '## Debug logging.

Set environment variable `RUST_LOG=debug` to see debug output.

This logs structured information about command line arguments and commands being run.

Recommend enabling debug logging for all demos to understand what is happening in more detail.
'

echo '```
$ RUST_LOG=debug rust-parallel echo ::: hi there how are you | grep command_line_args | head -1'
RUST_LOG=debug $RUST_PARALLEL echo ::: hi there how are you | grep command_line_args | head -1 | ansi-stripper

echo '
$ RUST_LOG=debug rust-parallel echo ::: hi there how are you | grep 'command_line_args:1''
RUST_LOG=debug $RUST_PARALLEL echo ::: hi there how are you | grep 'command_line_args:1' | ansi-stripper
echo '```'

echo '## Timeout.

The `-t` option can be used to specify a command timeout in seconds:
'

echo '```'

echo '$ rust-parallel -t 0.5 sleep ::: 0 3 5'
$RUST_PARALLEL -t 0.5 sleep ::: 0 3 5 | ansi-stripper

echo '```'

echo '## Progress bar.

The `-p` option can be used to enable a graphical progress bar.

This is best used for commands which are running for at least a few seconds, and which do not produce output to stdout or stderr.

In the below command `-d all` is used to discard all output from commands run:'

echo '```
$ rust-parallel -d all -p sleep ::: 1 2 3'
echo '⠤ [00:00:01] Commands Done/Total:  1/3  █████████░░░░░░░░░░░░░░░░░░ ETA 00:00:02'
echo '```'

echo '## Specifying command and initial arguments on command line:

Here `md5 -s` will be prepended to each input line to form a command like `md5 -s aal`
'

echo '```
$ head -100 /usr/share/dict/words | rust-parallel md5 -s | head -10'
head -100 /usr/share/dict/words | $RUST_PARALLEL md5 -s | head -10
echo '```
'

echo '## Using `awk` to form complete commands:'

echo '
```'
echo -e '$ head -100 /usr/share/dict/words | awk \x27{printf \x22md5 -s %s\\n\x22, $1}\x27 | rust-parallel | head -10'
head -100 /usr/share/dict/words | awk '{printf "md5 -s %s\n", $1}' | $RUST_PARALLEL | head -10
echo '```
'
echo '## Using as part of a shell pipeline.  

stdout and stderr from each command run are copied to stdout/stderr of the rust-parallel process.
'

echo '```
$ head -100 /usr/share/dict/words | rust-parallel md5 -s | grep -i abba'
head -100 /usr/share/dict/words | $RUST_PARALLEL md5 -s | grep -i abba
echo '```
'

echo '## Working on a set of files from `find` command.  

The `-0` option works nicely with `find -print0` to handle filenames with newline or whitespace characters:
'

rm -fr testdir

echo '```'
echo '$ mkdir testdir
'
mkdir testdir

echo "$ touch 'testdir/a b' 'testdir/b c' 'testdir/c d'
"
touch 'testdir/a b' 'testdir/b c' 'testdir/c d'

echo '$ find testdir -type f -print0 | rust-parallel -0 gzip -f -k
'
find testdir -type f -print0 | $RUST_PARALLEL -0 gzip -f -k

echo '$ ls -l testdir
'
ls -l testdir

echo '```
'

rm -fr testdir

echo '## Reading multiple inputs.

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

echo '
$ head -5 /usr/share/dict/words | rust-parallel -i - -i ./test echo'
head -5 /usr/share/dict/words | $RUST_PARALLEL -i - -i ./test echo

rm -f test

echo '```'

echo '## Bash Function

Use `-s` shell mode to invoke an arbitrary bash function.

Similar to normal commands bash functions can be called using stdin, input files, or from command line arguments.'

echo '### Function Setup

Define a bash fuction `logargs` that logs all arguments and make visible with `export -f`:
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

echo '```'

echo '### Demo of command line arguments:
'

echo '```
$ rust-parallel -s logargs ::: A B C ::: D E F'
$RUST_PARALLEL -s logargs ::: A B C ::: D E F

echo '```'

echo '### Demo of function and command line arguments from stdin:'

echo '```
$ cat >./test <<EOL
logargs hello alice
logargs hello bob
logargs hello charlie
EOL'
cat >./test <<EOL
logargs hello alice
logargs hello bob
logargs hello charlie
EOL

echo '
$ cat test | rust-parallel -s'

cat test | $RUST_PARALLEL -s
rm -f test

echo '```
'

echo '### Demo of function and initial arguments on command line, additional arguments from stdin:'

echo '```
$ cat >./test <<EOL
alice
bob
charlie
EOL'
cat >./test <<EOL
alice
bob
charlie
EOL

echo '
$ cat test | rust-parallel -s logargs hello'

cat test | $RUST_PARALLEL -s logargs hello
rm -f test

echo '```
'