# telegram-relay

`telegram-relay` forwards messages from the Telegram Bot API.
The relay will continously retrieve updates from the API and push json [Message](https://core.telegram.org/bots/api#message)s through sockets connected to
`localhost:9001`. It will automatically route messages depending on the sender of the message.

### Usage

`telegram-relay start <token>`

`token` is your bot's Telegram API token.

Listen on TCP port 9001 for line('\n') delimited json messages.

### Building

The Nightly release of Rust is currently required because the `Result.expect(&str)`
feature won't be included until Rust 1.4
