# SMTP Relay Server

Ever hosted some service which wanted to send mails but 

1. SMTP ports blocked by vps provider?
2. Dont wanna deal with SMTP/TLS/STARTTLS etc?

Or maybe you just want to recieve emails and send the contents as a json somewhere.

# Installation

Create a docker compose file for the service.

You can use the one provided in this repo as-is or make your own.

> The `docker-compose.yml` in the repo does not come with a port mapping on the coontainer because I would not recommend exposing this container to outside connections, as it lacks any form of authentication. Using docker networks is preferred. If you do expose the port, it is highly recommended to not expose it to the internet.

Only configuration required will be a `config.json` file at `/etc/smtp-relay` inside the container.

Example `config.json` file
```
{
  "smtp_port": 2525,
  "strategies": [
    {
      "type": "resend",
      "api_key": "re_Vgcz7RwG_AF1gNYxsiQF2MvsNVE8AiKgy",
      "from_address": "test@exmaple.com"
    },
    {
      "type": "webhook",
      "api_url": "exmaple.com",
      "from_address": "test@exmaple.com",
      "extra_headers": [
        "header": "value"
      ]
    }
  ]
}
```

# Strategies Available

1. Webhook
2. Resend.com

Currently, `ResendStrategy` is the only strategy to support file attachments and is decently tested. Webhooks are not really tested as they are not my primary usecase, although it might change in the future.

# Acknowledgments

https://github.com/nicolaihenriksen/SmtpToRestService