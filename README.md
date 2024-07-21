[![Latest Version](https://img.shields.io/crates/v/chatd)](https://crates.io/crates/chatd)
[![License: AGPL v3](https://img.shields.io/badge/License-AGPL_v3-blue.svg)](https://www.gnu.org/licenses/agpl-3.0)
[![Build Status](https://github.com/unrenamed/chatd/actions/workflows/build.yml/badge.svg?branch=main)](https://github.com/unrenamed/chatd/actions/workflows/build.yml)
[![Coverage Status](https://coveralls.io/repos/github/unrenamed/chatd/badge.svg)](https://coveralls.io/github/unrenamed/chatd)

# chatd

🚧 Work in Progress! 🚧

An implementation of an SSH server for real-time communication that
serves a chat room instead of a shell and provides encrypted messaging over
SSH.

### Acknowledgements

This project was greatly influenced by [shazow/ssh-chat](https://github.com/shazow/ssh-chat).

### Core Features

- [x] Public and private one-on-one conversations
- [x] Color themes
- [x] Built-in chat commands
- [x] Emacs-style key bindings
- [x] Command history
- [x] Configurable motd (message of the day)
- [x] Command autocomplete
- [x] Load user config overrides from ENV
- [ ] Automatically detect and handle idle users

### Security and Control

- [x] Option to allow connections from authorized users only
- [x] Messaging rate-limit to prevent spam
- [x] Special commands for operators (`/kick`, `/ban`, `/mute`, etc.)

### Quick start

```console
chatd is an implementation of an SSH server for real-time communication that
serves a chat room instead of a shell and provides encrypted messaging over
SSH.

Usage: chatd [OPTIONS]

Options:
      --port <PORT>       Port to listen on [default: 22]
  -i, --identity <KEY>    Private key to identify server with. Defaults to a temporary ed25519 key
      --oplist <FILE>     Optional file of public keys who are operators
      --whitelist <FILE>  Optional file of public keys who are allowed to connect
      --motd <FILE>       Optional file with a message of the day or welcome message
      --log <FILE>        Write chat log to this file
  -d, --debug...          Turn debugging information on
  -h, --help              Print help
  -V, --version           Print version
```

### Environment Variables

Due to the lack of persistent storage for user configurations in chatd (which is intentional), users need to reapply their settings each time they connect. This can be quite inconvenient, don't you think?

Using <b>environment variables</b> can solve this problem.

#### `CHATD_THEME`

This variable lets you set the theme for your session. Instead of manually configuring it by running `/theme hacker`, you can do it like this:

```bash
$ ssh -o SetEnv "CHATD_THEME=hacker" username@host
```

#### `CHATD_TIMESTAMP`

This variable enables the logging of a datetime or time prefix next to each received message. Instead of running `/timestamp datetime` manually, you can set it like this before connecting:

```bash
$ ssh -o SetEnv "CHATD_TIMESTAMP=datetime" username@host
```

If you find setting extra options to `ssh` command tiresome, you can use a configuration file supported by your ssh client. For the OpenSSH client, there is `.ssh/config` file. If you don't have one, feel free to create and provide r-w access `chmod 600 .ssh/config`.

Now add the following lines to the config file:

```bash
Host host
    SendEnv CHATD_*
```

Now, add the environment variables to your shell profile. Then, you can simply run:

```bash
$ ssh username@host
```

### Autocomplete

`chatd`'s autocomplete is designed to be intuitive and convenient for terminal users, recognizing whether you're completing a command, its subcommand, or their arguments.

For example:

<table>
<tr><td>[user] /opl</td><td>[user] /oplist</td></tr>
<tr><td>[user] /oplist l</td><td>[user] /oplist load</td></tr>
<tr><td>[user] /oplist load m</td><td>[user] /oplist load merge</td></tr>
<tr><td>[user] /oplist add al</td><td>[user] /oplist add alice</td></tr>
</table>

So, don't hesitate to press <kbd>Tab</kbd> whenever you want to save some typing 😉
