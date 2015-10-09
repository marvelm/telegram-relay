# telegram-relay

`telegram-relay` forwards messages from the Telegram Bot API.
The relay will continously retrieve updates from the API and push json messages through sockets connected to
`localhost:9001`. It will automatically route messages depending on the sender of the message.

### Usage

`telegram-relay start <token>`

`token` is your bot's Telegram API token.

Listen on TCP port 9001 for line('\n') delimited json messages.
