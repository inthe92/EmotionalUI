🇬🇧 English | [🇪🇸 Español](README.es.md)
# EmotionalUI
**Less searching. More playing.**

> **New functional beta:** [v0.2.0-beta.2](https://github.com/inthe92/EmotionalUI/releases/tag/v0.2.0-beta.2) · [Beta installation and limits](BETA.md). Blue/gold GPU atmosphere, game launching, resume and experimental offline Atlas. RG353V / dArkOS ARM64; separate Ports entry, no boot replacement. Installed on RG353V with files, loader and launcher planning checked; physical graphics/game validation remains pending. The [previous interface-only preview](https://github.com/inthe92/EmotionalUI/releases/tag/v0.1.0-preview.1) remains available.


### Games for every moment.

EmotionalUI was born on a rainy Sunday afternoon.

The lights were dim, soft music was playing in the background, and I had just finished a video game I'd been looking forward to playing for a long time.

It was one of those little moments when everything just feels right.

Then I returned to my console's menu.

I had thousands of games at my fingertips. Incredible adventures, unforgettable classics, and hidden gems I'd probably never even heard of.

I started scrolling through the list, moving from one game to another, unable to decide what to play next.

Eventually, I turned the console off.

**I had almost every video game in the world, yet I couldn't decide what to play.**

And that feeling became the beginning of EmotionalUI.

---

### A different way to experience video games

EmotionalUI is an alternative frontend for retro handhelds, designed to turn a collection of games into something more personal, welcoming, and meaningful.

It doesn't want to overwhelm you with hundreds of covers or convince you that you always need something new.

Instead, it helps you discover games that fit your tastes, your mood, and the moment you're in.

A rainy autumn afternoon. A quiet summer night. A slow Sunday with nowhere to be. Or simply a few minutes to unwind.

Every moment can be the beginning of a new adventure.

### Your adventures have a place here

EmotionalUI is designed to remember the games you love returning to, help you pick up where you left off, and shine a light on overlooked gems that deserve just as much attention as the classics.

Because a game doesn't have to be famous to be special.

And you don't have to finish every adventure for it to have been worth playing.

### An interface you can feel

Light, weather, seasons, music, and subtle visual details all become part of the experience.

We want turning on your handheld to feel like settling into your favorite cozy corner.

No rush. No unnecessary distractions. No endless menus that make you forget why you picked up your console in the first place.

**Technology should accompany the experience, not compete for your attention.**

### Your collection stays yours

EmotionalUI is designed to work with the games you already have, respecting your collection and everything you've built around it.

It's not a store. It's not a subscription service. It's not trying to sell you anything.

It's simply a different way to reconnect with your own games.

### An independent project, made with care

EmotionalUI is still in development. It started as a personal idea and is slowly growing into an exploration of how we discover, remember, and experience video games.

We don't know how far it will go.

But if you've ever found yourself surrounded by more games than you could ever play, yet unable to choose even one, you probably understand exactly why we're building it.

If this way of thinking about video games resonates with you, we'd love to hear your thoughts, ideas, or simply know that this project means something to you, too.

---

**EmotionalUI**

*It's not about having more games.*

**It's about giving them a meaningful place in our lives.**


## Experimental RG353V preview

The local frontend has run on a real Anbernic RG353V with dArkOS, including physical controls and a supervised return to EmulationStation. Artifact-specific installation and return results are recorded in the release notes.

The four sections are **Continue, Your moment, Library and Settings**. One room responds to local time, seasons, optional weather and editorial events. Offline operation does not require a weather service. Real weather requires a privately configured location and consent; demo weather is not an observation.

The `v0.1.0-preview.1` package uses **software by default** and offers **Mali-G52 / FemtoVG as experimental**. Immediate EGL swapping is not enabled by default. GPU improves perceived navigation in supervised tests, but stable 30 or 60 FPS have not been demonstrated. Text rasterization and some line breaks differ slightly.

Trial mode explores the interface and can read an existing library into its own SQLite database. **It cannot launch games.** No ROMs, BIOS, saves or commercial covers are bundled. Main frontend replacement (`install-main`) remains disabled.

[Installation and recovery](PREVIEW_INSTALL.md) · [Readiness and release notes](RELEASE_READINESS.md) · [Testing](TESTING.md) · [Licenses](THIRD_PARTY_LICENSES.md)

Rust + Slint 1.13.1; GPL-3.0-only for project code. Third-party fonts and dependencies retain their included notices. The owner has declared the room reference to be project-owned; its provenance is recorded. Wi-Fi/Bluetooth control and real game launching are not features of this trial preview.
