# The app ID is io.github.mmerkel.FreeCal

FreeCal's app ID is `io.github.mmerkel.FreeCal`, derived from its repository `github.com/mmerkel/FreeCal`. The ID names the data directory, the `.desktop` file, the icons, the D-Bus name and the Flatpak sandbox, so after the first release changing it means migrating every user's Local Store, secrets and desktop integration. It was settled before any release, while it was free to change. Flathub accepts a code-hosting ID when the repository URL derived from it is reachable, and ownership is shown through the GitHub account, so no domain is needed.

## Considered Options

- **`org.freecal.FreeCal`**, the placeholder used until then: `freecal.org` belongs to someone else, and Flathub expects a domain-based ID to match a domain the publisher controls.
- **A new domain such as `freecal.fr`**: costs a yearly renewal for as long as FreeCal exists, and verification needs a website on it.
- **The author's personal domain**: owned and free, but puts a personal name in every user's file system and stays tied to one person if others take the project over.

## Consequences

- `github.com/mmerkel/FreeCal` must be public by the Flathub submission, which comes after the v1 packages of ticket 28, and stay reachable. Moving the code elsewhere later needs the old URL to keep working or an exception from Flathub.
