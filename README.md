# ssh-chat

🚧 Work in Progress! 🚧

### Acknowledgements

This project was greatly influenced by [shazow/ssh-chat](https://github.com/shazow/ssh-chat).

### Core Features

- [x] Public and private one-on-one conversations
- [x] Color themes
- [x] Built-in chat commands
- [x] Emacs-style key bindings
- [x] Command history
- [x] Configurable motd (message of the day)
- [ ] Automatically detect and handle idle users

### Security and Control

- [x] Option to allow connections from authorized users only
- [x] Messaging rate-limit to prevent spam
- [x] Special commands for operators (`/kick`, `/ban`, `/mute`, etc.)

### Configuration

- [x] CLI for easy setup
- [ ] CI/CD _(optional)_
- [ ] Unit testing _(optional)_
- [ ] Benches and performance improvements _(optional)_

### Known Issues

1. When the prompt text is long enough to wrap to the next line(s), the cursor may not behave as expected.

2. When the prompt contains emoji or other Unicode characters, the cursor may not behave as expected.

### Quick start

```console
SSH Chat: Real-time communication over SSH

Usage: ssh-chat [OPTIONS]

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
