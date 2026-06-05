# Maintainer: Aner <argent0@github.com>
pkgname=nutlog
pkgver=0.1.0
pkgrel=1
pkgdesc="A Linux-first command-line nutrition logger with LLM agent skills support"
arch=('x86_64')
url="https://github.com/argent0/nutlog"
license=('custom')
depends=('gcc-libs')
provides=('nutlog')
makedepends=('git' 'rust' 'cargo')
source=("${pkgname}::git+ssh://git@github.com/argent0/nutlog.git")
sha256sums=('SKIP')

pkgver() {
  cd "$srcdir/$pkgname"
  local _ver=$(grep '^version =' Cargo.toml | head -n 1 | cut -d '"' -f 2)
  echo "${_ver}.r$(git rev-list --count HEAD).$(git rev-parse --short HEAD)"
}

build() {
  cd "$srcdir/$pkgname"
  cargo build --release --locked
}

package() {
  cd "$srcdir/$pkgname"
  install -Dm755 "target/release/nutlog" "$pkgdir/usr/bin/nutlog"
  install -Dm644 "README.md" "$pkgdir/usr/share/doc/$pkgname/README.md"
  install -Dm644 "AGENTS.md" "$pkgdir/usr/share/doc/$pkgname/AGENTS.md"
  install -Dm644 "CODING_PRACTICES.md" "$pkgdir/usr/share/doc/$pkgname/CODING_PRACTICES.md"

  # Detailed user and agent documentation
  install -d "$pkgdir/usr/share/doc/$pkgname/docs"
  install -Dm644 docs/*.md "$pkgdir/usr/share/doc/$pkgname/docs/"

  # LICENSE is TBD per README.md (personal tool for now); no license file installed

  # LLM agent documentation (AGENTS.md, CODING_PRACTICES.md and docs/) are installed
  # to support LLM agents using the tool, analogous to skills in similar projects.
}
