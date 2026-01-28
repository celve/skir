PREFIX ?= $(HOME)/.local
BINDIR ?= $(PREFIX)/bin

.PHONY: build release install uninstall clean

build:
	cargo build

release:
	cargo build --release

install: release
	mkdir -p $(BINDIR)
	cp target/release/skir $(BINDIR)/skir

uninstall:
	rm -f $(BINDIR)/skir

clean:
	cargo clean
