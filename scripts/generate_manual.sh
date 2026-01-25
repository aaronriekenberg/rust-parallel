#!/bin/bash

set -e

export NO_COLOR=1
RUST_PARALLEL="./target/debug/rust-parallel"
VERSION=$($RUST_PARALLEL -V | cut -f2 -d' ')

echo "## Manual for rust-parallel $VERSION"

echo '
1. [Command line options](#command-line-options)
1. [Commands from arguments](#commands-from-arguments)
   1. [Automatic variables](#automatic-variables)
1. [Commands from stdin](#commands-from-stdin)
1. [Command and initial arguments on command line](#command-and-initial-arguments-on-command-line)
1. [Reading multiple inputs](#reading-multiple-inputs)
1. [Pipe Mode](#pipe-mode)
1. [Parallelism](#parallelism)
1. [Keep Output Order](#keep-output-order)
1. [Dry run](#dry-run)
1. [Debug logging](#debug-logging)
1. [Error handling](#error-handling)
1. [Timeout](#timeout)
1. [Path cache](#path-cache)
1. [Progress bar](#progress-bar)
1. [Regular Expression](#regular-expression)
   1. [Named Capture Groups](#named-capture-groups)
   1. [Numbered Capture Groups](#numbered-capture-groups)
   1. [Capture Group Special Characters](#capture-group-special-characters)
1. [Shell Commands](#shell-commands)
1. [Bash Function](#bash-function)
   1. [Function Setup](#function-setup)
   1. [Demo of command line arguments](#demo-of-command-line-arguments)
   1. [Demo of function and command line arguments from stdin](#demo-of-function-and-command-line-arguments-from-stdin)
   1. [Demo of function and initial arguments on command line, additional arguments from stdin](#demo-of-function-and-initial-arguments-on-command-line-additional-arguments-from-stdin)
'

echo '## Command line options'

echo '```
$ rust-parallel --help'
$RUST_PARALLEL --help
echo '```'

echo '## Commands from arguments

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

echo '### Automatic Variables'

echo 'When using commands from arguments, numbered variables `{0}`, `{1}`, etc are automatically available based on the number of arguments.  `{0}` will be replaced by the entire input line, and other groups match individual argument groups.  `{}` is the same as `{0}`.  This is useful for building more complex command lines.  For example:
'

echo '```
$ rust-parallel echo group0={0} group1={1} group2={2} group3={3} group2again={2} ::: A B ::: C D ::: E F G'
$RUST_PARALLEL echo group0={0} group1={1} group2={2} group3={3} group2again={2} ::: A B ::: C D ::: E F G
echo '```'

echo '```
$ rust-parallel echo entireline={} group1={1} group2={2} group3={3} group2again={2} ::: A B ::: C D ::: E F G'
$RUST_PARALLEL echo entireline={} group1={1} group2={2} group3={3} group2again={2} ::: A B ::: C D ::: E F G
echo '```'

echo 'Internally these variables are implemented using an auto-generated [regular expression](#regular-expression).  If a regular expression is manually specified this will override the auto-generated one.'

echo '## Commands from stdin

Run complete commands from stdin.

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
$ cat test | rust-parallel'
cat test | $RUST_PARALLEL

rm -f test

echo '```'

echo '## Command and initial arguments on command line

Here `md5 -s` will be prepended to each input line to form a command like `md5 -s aal`
'

echo '```
$ head -100 /usr/share/dict/words | rust-parallel md5 -s | head -10'
head -100 /usr/share/dict/words | $RUST_PARALLEL md5 -s | head -10
echo '```
'

echo '## Reading multiple inputs

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

echo '## Pipe Mode

The `--pipe` option can be used to enable pipe mode.

In pipe mode input from stdin is split into blocks and each block is passed to a separate instance of the command via stdin.  Command instances are run in parallel.  

The default block size is 1 MiB, which can be changed with the `--block-size` option.

By default input blocks are split on new line boundaries.  This can be changed to split on null boundaries with the `-0/--null-separator` option.

Here we use `--pipe` to run `wc -l`
'

echo '```'
echo '$ cat /usr/share/dict/words | rust-parallel --pipe wc -l'
cat /usr/share/dict/words | $RUST_PARALLEL --pipe wc -l
echo '```'

echo 'Here we use pipe mode with with a smaller block size of 500 KiB:'

echo '```'
echo '$ cat /usr/share/dict/words | rust-parallel --pipe --block-size=500KiB wc -l'
cat /usr/share/dict/words | $RUST_PARALLEL --pipe --block-size=500KiB wc -l
echo '```'


echo '
## Parallelism

By default the number of parallel jobs to run simulatenously is the number of cpus detected at run time.

This can be override with the `-j`/`--jobs` option.

With `-j5` all echo commands below run in parallel.

With `-j1` all jobs run sequentially.
'

echo '```
$ rust-parallel -j5 echo ::: hi there how are you'
$RUST_PARALLEL -j5 echo ::: hi there how are you

echo '
$ rust-parallel -j1 echo ::: hi there how are you'
$RUST_PARALLEL -j1 echo ::: hi there how are you

echo '```'

echo '## Keep Output Order

By default, command outputs are displayed as soon as each command completes, which may not be in the same order as the input.

Use option `-k`/`--keep-order` to ensure outputs are displayed in the same order as the input.

With `-k` all outputs will be displayed in the same order as the input, regardless of when commands complete.
'

echo '```
$ rust-parallel -k echo ::: hi there how are you'
$RUST_PARALLEL -k echo ::: hi there how are you

echo '
$ rust-parallel -j1 -k echo ::: hi there how are you'
$RUST_PARALLEL -j1 -k echo ::: hi there how are you

echo '```'

echo '## Dry run

Use option `--dry-run` for dry run mode.

In this mode the commands that would be run are ouput as info level logs.

No commands are actually run - this is useful for testing before running a job.
'

echo '```
$ rust-parallel --dry-run echo ::: hi there how are you'
$RUST_PARALLEL --dry-run echo ::: hi there how are you
echo '```'

echo '## Debug logging

Set environment variable `RUST_LOG=debug` to see debug output.

This logs structured information about command line arguments and commands being run.

Recommend enabling debug logging for all examples to understand what is happening in more detail.
'

echo '```
$ RUST_LOG=debug rust-parallel echo ::: hi there how are you | grep command_line_args | head -1'
RUST_LOG=debug $RUST_PARALLEL echo ::: hi there how are you | grep command_line_args | head -1

echo '
$ RUST_LOG=debug rust-parallel echo ::: hi there how are you | grep 'command_line_args:1''
RUST_LOG=debug $RUST_PARALLEL echo ::: hi there how are you | grep 'command_line_args:1'
echo '```'

echo '## Error handling

The following are considered command failures and error will be logged:
* Spawn error
* Timeout
* I/O error
* Command exits with non-0 status

By default rust-parallel runs all commands even if failures occur.

When rust-parallel terminates, if any command failed it logs failure metrics and exits with status 1.

Here we try to use `cat` to show non-existing files `A`, `B`, and `C`, so each command exits with status 1:
'

echo '```
$ rust-parallel cat ::: A B C'
set +e
$RUST_PARALLEL cat ::: A B C 2>&1
RET_VAL=$?
set -e

echo '
$ echo $?'
echo $RET_VAL
echo '```'

echo 'The `--exit-on-error` option can be used to exit after one command fails.

rust-parallel waits for in-progress commands to finish before exiting and then exits with status 1.'
echo '```
$ head -100 /usr/share/dict/words | rust-parallel --exit-on-error cat'
set +e
head -100 /usr/share/dict/words | $RUST_PARALLEL --exit-on-error cat 2>&1
RET_VAL=$?
set -e

echo '
$ echo $?'
echo $RET_VAL
echo '```'

echo '
## Timeout

The `-t`/`--timeout-seconds` option can be used to specify a command timeout in seconds.  If any command times out this is considered a command failure (see [error handling](#error-handling)).
'

echo '```'

echo '$ rust-parallel -t 0.5 sleep ::: 0 3 5'
set +e
$RUST_PARALLEL -t 0.5 sleep ::: 0 3 5
RET_VAL=$?
set -e

echo '
$ echo $?'
echo $RET_VAL
echo '```'

echo '
## Path Cache

By default as commands are run the full paths are resolved using [which](https://github.com/harryfei/which-rs).  Resolved paths are stored in a cache to prevent duplicate resolutions.  This is generally [good for performance](https://github.com/aaronriekenberg/rust-parallel/wiki/Benchmarks).

The path cache can be disabled using the `--disable-path-cache` option.
'

echo '## Progress bar

The `-p`/`--progress-bar` option can be used to enable a graphical progress bar.

This is best used for commands which are running for at least a few seconds, and which do not produce output to stdout or stderr.  In the below commands `-d all` is used to discard all output from commands run.

Progress styles can be chosen with the `PROGRESS_STYLE` environment variable.  If `PROGRESS_STYLE` is not set it defaults to `light_bg`.

The following progress styles are available:
* `PROGRESS_STYLE=light_bg` good for light terminal background with colors, spinner, and steady tick enabled:
![light_bg](https://github.com/aaronriekenberg/rust-parallel/blob/main/screenshots/light_background_progress_bar.png)

* `PROGRESS_STYLE=dark_bg` good for dark terminal background with colors, spinner, and steady tick enabled:
![dark_bg](https://github.com/aaronriekenberg/rust-parallel/blob/main/screenshots/dark_background_progress_bar.png)

* `PROGRESS_STYLE=simple` good for simple or non-ansi terminals/jobs with colors, spinner, and steady tick disabled:
![simple](https://github.com/aaronriekenberg/rust-parallel/blob/main/screenshots/simple_progress_bar.png)

## Regular Expression

Regular expressions can be specified by the `-r` or `--regex` command line argument.

[Named or numbered capture groups](https://docs.rs/regex/latest/regex/#grouping-and-flags) are expanded with data values from the current input before the command is executed.

### Named Capture Groups

In these examples using command line arguments `{url}` and `{filename}` are named capture groups.  `{}` is a variable meaning the entire input line.
'

echo '```'
echo -e '$ rust-parallel -r \x27(?P<url>.*),(?P<filename>.*)\x27 echo got url={url} filename={filename} ::: URL1,filename1 URL2,filename2'
$RUST_PARALLEL -r '(?P<url>.*),(?P<filename>.*)' echo got url={url} filename={filename} ::: URL1,filename1 URL2,filename2

echo
echo -e '$ rust-parallel -r \x27(?P<url>.*) (?P<filename>.*)\x27 echo got url={url} filename={filename} full input={} ::: URL1 URL2 ::: filename1 filename2'
$RUST_PARALLEL -r '(?P<url>.*) (?P<filename>.*)' echo got url={url} filename={filename} full input={} ::: URL1 URL2 ::: filename1 filename2

echo '```'

echo '### Numbered Capture Groups

In the next example input file arguments `{1}` `{2}` `{3}` are numbered capture groups, `{}` is a variable meaning the entire input line.  The input is a csv file:'

echo '```'
echo '$ cat >./test <<EOL
foo,bar,baz
foo2,bar2,baz2
foo3,bar3,baz3
EOL'
cat >./test <<EOL
foo,bar,baz
foo2,bar2,baz2
foo3,bar3,baz3
EOL

echo
echo -e '$ cat test | rust-parallel -r \x27(.*),(.*),(.*)\x27 echo got arg1={1} arg2={2} arg3={3} full input={}'
cat test | $RUST_PARALLEL -r '(.*),(.*),(.*)' echo got arg1={1} arg2={2} arg3={3} full input={}

echo '```'

rm -f test

echo '### Capture Group Special Characters

All occurrences of capture groups are replaced as exact strings.  Surrounding characters have no effect on this.

This means capture groups can be nested with other `{` or `}` characters such as when building json:'

echo '```'
echo '$ cat >./test <<EOL
1,2,3
4,5,6
7,8,9
EOL'
cat >./test <<EOL
1,2,3
4,5,6
7,8,9
EOL

echo
echo -e '$ cat test | rust-parallel -r \x27(.*),(.*),(.*)\x27 echo \x27{"one":{1},"two":{2},"nested_object":{"three":{3}}}\x27'
cat test | $RUST_PARALLEL -r '(.*),(.*),(.*)' echo '{"one":{1},"two":{2},"nested_object":{"three":{3}}}'

echo '```'

rm -f test

echo '## Shell Commands

Shell commands can be written using `-s` shell mode.

Multiline commands can be written using `;`.

Environment variables, `$` characters, nested commands and much more are possible:'

echo '```'
echo -e '$ rust-parallel -s -r \x27(?P<arg1>.*) (?P<arg2>.*)\x27 \x27FOO={arg1}; BAR={arg2}; echo "FOO = ${FOO}, BAR = ${BAR}, shell pid = $$, date = $(date)"\x27 ::: A B ::: CAT DOG'
$RUST_PARALLEL -s -r '(?P<arg1>.*) (?P<arg2>.*)' 'FOO={arg1}; BAR={arg2}; echo "FOO = ${FOO}, BAR = ${BAR}, shell pid = $$, date = $(date)"' ::: A B ::: CAT DOG
echo '```'

echo '## Bash Function

`-s` shell mode can be used to invoke an arbitrary bash function.

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

echo '```'
