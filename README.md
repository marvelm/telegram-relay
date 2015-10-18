# telegram-relay

`telegram-relay` forwards messages from the Telegram Bot API.
The relay will continously retrieve updates from the API and push JSON messages to
`localhost:9001`. It will automatically route messages depending on the sender of the message.
`telegram-relay` will make sure that a sender is always associated with a single listener.

TODO: Make the relay re-route messages bound for disconnected listeners.

### Usage

`telegram-relay start <token>`

`token` is your bot's Telegram API token.

Listen on TCP port 9001 for line ('\n') delimited json messages. There will be one
[Message](https://core.telegram.org/bots/api#message) per line.

### Building

The Nightly release of Rust is currently required because the `Result.expect(&str)`
feature won't be included until Rust 1.4 and I'm too lazy to do `Result.ok().expect(&str)`
