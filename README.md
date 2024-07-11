# chatd

🚧 Work in Progress! 🚧

SSH chat server written in Rust 🦀 for real-time communication, providing encrypted messaging over SSH.

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
SSH Chat: Real-time communication over SSH

Usage: chatd [OPTIONS]

Options:
      --port <PORT>       Port to listen on [default: 2222]
  -i, --identity <KEY>    Private key to identify server with. Defaults to a temporary ed25519 key
      --oplist <FILE>     Optional file of public keys who are operators
      --whitelist <FILE>  Optional file of public keys who are allowed to connect
      --motd <FILE>       Optional file with a message of the day or welcome message
      --log <FILE>        Write chat log to this file
  -d, --debug...          Turn debugging information on
  -h, --help              Print help
  -V, --version           Print version
```
