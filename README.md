
# Kattis Client


## Features


## Setup

### Installation

#### Using Cargo

```
git clone https://github.com/nolanderc/kattis-client
cd kattis-client
cargo install --path .
```

> It is normal for compilation to take a couple minutes. Meanwhile, you
> can prepare your [configuration](#configuration).

## Example Usage

To illustrate how this tool may be used, we are going to solve the problem
[Aaah](https://openkattis.com/problems/aaah). You can find the configuration and
files used in this example in the
[examples](https://github.com/nolanderc/kattis-client/tree/master/examples)
directory.

If you don't already have one, you have to create a new [template](#templates).
A template is basically a folder containing a barebones solution with all
boilerplate code already typed out. Find out more [here](#templates).

Our first order of business is to find the problem's id: `aaah`. This will be
used by the client to download all the problem's samples and setup a new test
environment:

```
kattis new --problem aaah --template rust
```

By default this command creates a new directory called `aaah` in which we will
find our template (in this case the template called `rust`). 

At this point we would write our solution, and when we believe we have a working
solution we can test our hypothesis by running:

```
cd aaah
kattis test
```

This will run the build commands specified in the `kattis.yml` followed by the
run commands, once for every sample. If our solution was correct we should see
the following output:

```
Running test case: aaah.1
Correct
Running test case: aaah.2
Correct
```

If our solution was wrong we would instead see something like (actual output is
colorized for readability):

```
Running test case: aaah.1
Correct
Running test case: aaah.2
Wrong Answer

Input:
aaah
ah

Found:
uh oh

Expected:
go
```

Testing your code before submitting will not only make debugging easier, but
will also reduce the possibility of you getting a test case wrong. This is
especially important in a competition where a wrong answer includes a penalty.

When you are confident that your solution is correct you can easily submit your
solution by running:

```
kattis submit
```

It will prompt you with a short summary of what will be submitted along with a
confirmation prompt. If you answer `y` your submission will be sent and you
should see the status of your submission get periodically updated:

```
Language: Rust
Files:
  - ./main.rs
Main Class:
Proceed with the submission? (y/N) y
Submission ID: 4253057
Compiling...
Running...
Test Case 1/52: Accepted
Test Case 2/52: Accepted
Test Case 3/52: Accepted
Test Case 4/52: Accepted
...
Test Case 42/52: Accepted
...
Test Case 49/52: Accepted
Test Case 50/52: Accepted
Test Case 51/52: Accepted
Test Case 52/52: Accepted

Submission Status: Accepted
Time: 17:47:24
CPU: 0.00 s
```

As you would expect, if something has gone terribly wrong you will find out ASAP:

```
Language: Rust
Files:
  - ./main.rs
Main Class:
Proceed with the submission? (y/N) y
Submission ID: 4253066
Compiling...
Compiling...
Test Case 1/52: Accepted
Test Case 2/52: Accepted
Test Case 3/52: Accepted
Test Case 4/52: Accepted
Test Case 5/52: Wrong Answer

Submission Status: Wrong Answer
Time: 17:53:36
CPU: 0.00 s
```


## Configuration

By default kattis will search for your configuration files in your user's
configuration directory:

| Operating System | Directory | Example |
| --- | --- | --- |
| Linux | $XDG_CONFIG_HOME/kattis or $HOME/.config/kattis | /home/alice/.config/kattis |
| macOS | $HOME/Library/Preferences/kattis | /Users/Alice/Library/Preferences/kattis |
| Linux | {FOLDERID_RoamingAppData} | C:\Users\Alice\AppData\Roaming\kattis |

You can override the default configuration directory by setting the 
`KATTIS_CONFIG_HOME` environment variable.

Inside the configuration directory you can create a global configuration file
`kattis-global.yml`. You may print the path to this file by running `kattis
config show`

### Credentials

In order to make submissions from the command line you will need to download
the appropriate credentials. These can be found
[here](https://open.kattis.com/donload/kattisrc). If you want to use this
client on any other domain than [open.kattis.com](https://open.kattis.com)
you will find credentials at `https://<kattis>/download/kattisrc` where
`<kattis>` is the domain, eg. `https://po.kattis.com/download/kattisrc`.

Unlike the official submission CLI this client does not expect credentials to be
stored in your home directory. Instead they are stored the `credentials` folder
inside your configuration directory.

You may name the file containing the credentials whatever you want. Later, such
when as when making a new submission, you can use this name to tell the client
to submit to a different Kattis domain. This can, among other times, be useful
during competitions.

By default the client will search for credentials with the name
`open.kattis.com`. This behaviour can be overriden by changing the
`default_hostname` property in either the global configuration file
(`kattis-global.yml`), the solution's configuration file (`kattis.yml`) or with
a command line flag.

### Templates

A template is basically a folder containing a barebones solution with all
boilerplate code already typed out. They can save you a lot of time during
competitions (when allowed) as you can spend more time on the problem at hand
and less typing code.

Templates are stored as seperate directories in the `templates` folder inside
the configuration directory. It is simple to create a new template:

```
kattis template new <name>
```

This will create a new template called `<name>` in the appropriate directory and
create a default `kattis.yml` configuration file. 

By default you will have to provide the client with the name of the template you
wish to use with a flag. This behaviour can be overriden by changing the
`default_template` property in either the global configuration file
(`kattis-global.yml`).

#### The `kattis.yml` file

This YAML file you may configure how the template is built, which files are
submitted, and so on:

| Field       | Description                                                  |
| -----       | -----------                                                  |
| `samples`   | The directory in which samples will be stored                |
| `files`     | A list of files that should be submitted to the judge        |
| `language`  | The language the solution is written in                      |
| `mainclass` | Optional. Specify the main class                             |
| `build`     | A list of commands to execute in order to build the solution |
| `run`       | The command to run in order to run the solution              |

> Note that for interpreted languages such as Python there's no need for a build
> step, as such you may leave it empty.

When a template is used to create a new solution to a problem using the `kattis
new` command two additional fields are created:

| Field      | Description                                        |
| -----      | -----------                                        |
| `hostname` | The name of the credentials to use when submitting |
| `problem`  | The id of the problem the solution solves          |

