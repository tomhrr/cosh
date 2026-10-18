prefix=/usr/local
bindir=$(prefix)/bin
libdir=$(prefix)/lib
INSTALL=install
UNAME_S := $(shell uname -s)
ifeq ($(UNAME_S),Darwin)
    $(warning Using 'ginstall' on macOS. If not available install coreutils, e.g., $ brew install coreutils.)
	INSTALL=ginstall
endif

SOURCE_FILES := $(wildcard src/*.rs src/bin/*.rs src/vm/*rs)

all: rt.chc rdap.chc rpkiv.chc rpsl.chc ssh-agent.chc nrtm.chc getopts.chc

rt.chc: target/release/cosh lib/rt.ch
	./target/release/cosh --no-rt -o rt.chc -c lib/rt.ch

rdap.chc: target/release/cosh rt.chc lib/rdap.ch
	./target/release/cosh -o rdap.chc -c lib/rdap.ch

rpkiv.chc: target/release/cosh rt.chc lib/rpkiv.ch
	./target/release/cosh -o rpkiv.ch -c lib/rpkiv.ch

rpsl.chc: target/release/cosh rt.chc lib/rpsl.ch
	./target/release/cosh -o rpsl.chc -c lib/rpsl.ch

ssh-agent.chc: target/release/cosh rt.chc lib/ssh-agent.ch
	./target/release/cosh -o ssh-agent.chc -c lib/ssh-agent.ch

nrtm.chc: target/release/cosh rt.chc lib/nrtm.ch
	./target/release/cosh -o nrtm.chc -c lib/nrtm.ch

getopts.chc: target/release/cosh rt.chc lib/getopts.ch
	./target/release/cosh -o getopts.chc -c lib/getopts.ch

target/release/cosh: $(SOURCE_FILES)
	libdir=$(libdir) cargo build --release

test:
	cargo test --release -- --nocapture

install: rt.chc
	$(INSTALL) -D -m 755 target/release/cosh $(bindir)/cosh
	$(INSTALL) -D -m 755 rt.chc -t $(libdir)/cosh/
	$(INSTALL) -D -m 755 rdap.chc -t $(libdir)/cosh/
	$(INSTALL) -D -m 755 rpkiv.chc -t $(libdir)/cosh/
	$(INSTALL) -D -m 755 rpsl.chc -t $(libdir)/cosh/
	$(INSTALL) -D -m 755 ssh-agent.chc -t $(libdir)/cosh/
	$(INSTALL) -D -m 755 nrtm.chc -t $(libdir)/cosh/
	$(INSTALL) -D -m 755 getopts.chc -t $(libdir)/cosh/

clean:
	rm *.chc

clean-all: clean
	rm -rf target
