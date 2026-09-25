PREFIX ?= /usr/local
BINDIR ?= $(PREFIX)/bin
SHAREDIR ?= $(PREFIX)/share

.PHONY: all build release install uninstall deb clean

all: build

build:
	cargo build --release

install: build
	install -Dm755 target/release/pulsarkey $(DESTDIR)$(BINDIR)/pulsarkey
	ln -sf $(BINDIR)/pulsarkey $(DESTDIR)$(BINDIR)/cosmic-fido2
	install -Dm755 target/release/pulsarkey-settings $(DESTDIR)$(BINDIR)/pulsarkey-settings
	install -Dm644 packaging/cosmic-fido2.svg $(DESTDIR)$(SHAREDIR)/icons/hicolor/scalable/apps/io.github.mzia.PulsarKey.svg
	install -Dm644 packaging/icons/auth-fingerprint-symbolic.svg $(DESTDIR)$(SHAREDIR)/icons/hicolor/scalable/status/auth-fingerprint-symbolic.svg
	install -Dm644 packaging/icons/auth-fingerprint-disconnected-symbolic.svg $(DESTDIR)$(SHAREDIR)/icons/hicolor/scalable/status/auth-fingerprint-disconnected-symbolic.svg
	install -Dm644 packaging/flatpak/io.github.mzia.PulsarKey.desktop $(DESTDIR)$(SHAREDIR)/applications/io.github.mzia.PulsarKey.desktop
	install -Dm644 packaging/flatpak/io.github.mzia.PulsarKey.metainfo.xml $(DESTDIR)$(SHAREDIR)/metainfo/io.github.mzia.PulsarKey.metainfo.xml

uninstall:
	rm -f $(DESTDIR)$(BINDIR)/pulsarkey
	rm -f $(DESTDIR)$(BINDIR)/cosmic-fido2
	rm -f $(DESTDIR)$(BINDIR)/pulsarkey-settings
	rm -f $(DESTDIR)$(SHAREDIR)/icons/hicolor/scalable/apps/io.github.mzia.PulsarKey.svg
	rm -f $(DESTDIR)$(SHAREDIR)/icons/hicolor/scalable/status/auth-fingerprint-symbolic.svg
	rm -f $(DESTDIR)$(SHAREDIR)/icons/hicolor/scalable/status/auth-fingerprint-disconnected-symbolic.svg
	rm -f $(DESTDIR)$(SHAREDIR)/applications/io.github.mzia.PulsarKey.desktop
	rm -f $(DESTDIR)$(SHAREDIR)/metainfo/io.github.mzia.PulsarKey.metainfo.xml

deb: build
	chmod +x packaging/build_deb.sh
	./packaging/build_deb.sh

clean:
	cargo clean
	rm -rf packaging/deb/pulsarkey_* packaging/*.deb dist/*.deb
