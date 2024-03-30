prefix=/usr/local
bindir=$(prefix)/bin
libdir=$(prefix)/lib
INSTALL=install
UNAME_S := $(shell uname -s)
ifeq ($(UNAME_S),Darwin)
    $(warning Using 'ginstall' on macOS. If not available install coreutils, e.g., $ brew install coreutils.)
	INSTALL=ginstall
endif

all: rt.chc rdap.chc

rt.chc: target/release/cosh lib/rt.ch
	./target/release/cosh --no-rt -c lib/rt.ch -o rt.chc

rdap.chc: target/release/cosh rt.chc lib/rdap.ch
	./target/release/cosh -c lib/rdap.ch -o rdap.chc

target/release/cosh:
	libdir=$(libdir) cargo build --release

test:
	cargo test --release

install: rt.chc
	$(INSTALL) -D -m 755 target/release/cosh $(bindir)/cosh
	$(INSTALL) -D -m 755 rt.chc -t $(libdir)/cosh/
	$(INSTALL) -D -m 755 rdap.chc -t $(libdir)/cosh/

clean:
	rm *.chc

clean-all: clean
	rm -rf target
