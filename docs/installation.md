# Installation

## From Source (Cargo)

```bash
git clone https://github.com/argent0/nutlog.git
cd nutlog
cargo install --path .
```

The binary `nutlog` will be placed in `~/.cargo/bin/` (ensure on `$PATH`).

For development:

```bash
cargo run -- product list --json
```

## Arch Linux / AUR (PKGBUILD)

A `PKGBUILD` is provided in the repository root for building a package (e.g. for Arch User Repository).

Typical build:

```bash
makepkg -si
```

The package:

- Installs the release binary to `/usr/bin/nutlog`
- Installs documentation (including this `docs/` tree) to `/usr/share/doc/nutlog/`

See the [PKGBUILD](../PKGBUILD) for exact files installed and the `pkgver()` logic (includes git rev count for development snapshots).

## Binary Only

Pre-built binaries are not currently provided. Build from source or use the package for your distro.

## Verifying Installation

```bash
nutlog --version
nutlog --help
```

On first use any command will create the database and run migrations (including pre-populating standard nutrients).

## Uninstallation

- Cargo: `cargo uninstall nutlog`
- Package manager: use your distro's package removal command (e.g. `pacman -R nutlog` if installed via AUR helper).

The database (`nutlog.db`) is **not** removed automatically — it lives in your XDG data directory.

## Documentation Location (Installed)

When installed via package:

- Top-level docs: `/usr/share/doc/nutlog/README.md`, `AGENTS.md`, `CODING_PRACTICES.md`
- Detailed docs: `/usr/share/doc/nutlog/docs/*.md`

You can read them with any pager:

```bash
less /usr/share/doc/nutlog/docs/index.md
man -l /usr/share/doc/nutlog/docs/index.md   # (won't be perfect without man conversion)
```

## Dependencies (Runtime)

- glibc / gcc-libs (standard on Linux)
- No other runtime dependencies (SQLite is statically bundled via `rusqlite` "bundled" feature).

Build-time: Rust toolchain + cargo.

## Next Steps

See [Getting Started](getting-started.md).
