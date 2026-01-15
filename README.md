![banner](./banner.png)
# <img src="./logo.png" alt="logo" width=40 height=25>ImposterBot 

ImposterBot is a feature-rich Discord bot written in **Rust**, focused on fun interactions, voice chat playback, and Minecraft server management. It is designed to be modular, extensible, safe, and reliable.

---

## Features

### Fun Commands
Lightweight commands for casual interaction:
- **`roll <sides>`** – Roll a die with any number of sides
- **`coinflip`** – Flip a coin (heads or tails)

---

### Voice Chat Commands *(feature-gated)*
Voice commands allow the bot to join voice channels and play audio:
- **`play mariah`** – Joins the voice channel and plays Mariah Carey Christmas music 🎄
- **`play youtube <url | search>`** – Plays audio from a YouTube link or search term
- **`play stop`** – Stops playback and leaves the voice channel

> Voice support is optional and controlled via cargo feature flags ("voice" or "youtube").

---

### Minecraft Server Advertising
Manage and advertise Minecraft servers directly from Discord:
- **`mc status`** – Get the current status of an advertised server
- **`mc add`** – Add a new Minecraft server
- **`mc remove`** – Remove an existing server
- **`mc update`** – Update server information

---

### Member Management
Automate and customize member onboarding:
- **`configure_welcome_channel`** – Set the channel for welcome and goodbye messages
- **`add_default_member_role`** – Add a role automatically assigned to new members
- **`remove_default_member_role`** – Remove a role from the auto-assigned list

---

## Technologies

- [**Rust**](https://rust-lang.org/)
- **[Serenity](https://github.com/serenity-rs/serenity) / [Poise](https://github.com/serenity-rs/poise)** for Discord interactions
- **[Songbird](https://github.com/serenity-rs/songbird)** for voice support
- **Tokio** for async runtime
- **Anyhow** for error handling
- **Tracing** for structured logging

---

## 🚀 Getting Started

### Prerequisites
- Rust (1.92.0 recommended)
- A Discord bot token
- (Optional) [cmake](https://cmake.org/download/), c compiler for voice feature 
- (Optional) [yt-dlp](https://github.com/yt-dlp/yt-dlp) for youtube feature

---

### Clone the Repository

```bash
git clone https://github.com/calebberhow/Imposterbot.RS
cd Imposterbot.RS
```

### Configuration

Create `.env` file, and enter your discord token.

```
DISCORD_TOKEN=<your token here>
COMMAND_DISABLE_LIST=
LOG_LEVEL=warn,imposterbot=trace
LOG_PATH=true
OWNERS=
DATABASE_URL=sqlite:./data/imposterbot-data.db?mode=rwc
CMAKE_CONFIGURE_ARGS="-CMAKE_POLICY_VERSION_MINIMUM=3.5"
```

### Running the Bot

`cargo run --release`

Or with voice enabled (additional developer dependencies required):

`cargo run --release --features="voice"`

Or with voice and youtube playback enabled (additional developer dependencies required):

`cargo run --release --features="youtube"`

Or with docker (youtube feature enabled automatically without requiring dev dependencies)

`docker compose up -d --build`

## 🤝 Contributing

Contributions are welcome!

### Guidelines

- Follow standard Rust formatting

- Prefer feature-gated additions when introducing functionality with heavy dependencies (like opus/openssl/python/yt-dlp/etc for `voice` feature)

- Write clear commit messages

### Suggested Improvements

Additional fun or utility commands

Expanded Minecraft server integrations

Improved audio queueing and playlists

Better test coverage for non-Discord logic

### Project Structure

```
src/
├── main.rs          # Application entry point
├── client.rs        # Discord client and framework setup
├── database.rs      # Database initialization and access
├── logging.rs       # Logging configuration
├── shutdown.rs      # Graceful shutdown and signal handling
├── commands/        # Bot commands
├── events/          # Functionality that requires hooking into serenity event system
├── infrastructure/  # Useful stuff not necessarily tied to a particular feature
```

## License

This project is licensed under the MIT License.
See the [LICENSE](./license.md) file for details.
