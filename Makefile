PREFIX ?= $(HOME)/.local
BINDIR ?= $(PREFIX)/bin

.PHONY: build release install uninstall clean

build:
	cargo build

release:
	cargo build --release

install: release
	mkdir -p $(BINDIR)
	cp target/release/silk $(BINDIR)/silk

uninstall:
	rm -f $(BINDIR)/silk

clean:
	cargo clean
