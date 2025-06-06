# Discord Message Scheduler Bot

```yaml
services:
  message-scheduler-bot:
    image: beanbeanjuice/discord-message-scheduler-bot:release
    restart: unless-stopped
    volumes:
      - /some/custom/dir:/app/data
    environment:
      BOT_TOKEN: ${MESSAGE_SCHEDULER_BOT_TOKEN}
```
