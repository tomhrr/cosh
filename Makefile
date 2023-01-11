prefix=/usr/local
bindir=$(prefix)/bin
libdir=$(prefix)/lib

all: rt.chc

rt.chc: target/release/cosh
	./target/release/cosh --no-rt -c lib/rt.ch -o rt.chc

target/release/cosh:
	libdir=$(libdir) cargo build --release

test:
	cargo test --release

install: rt.chc
	install -D -m 755 bin/wrapped-cosh $(bindir)/wrapped-cosh
	install -D -m 755 target/release/cosh $(bindir)/cosh
	install -D -m 755 rt.chc -t $(libdir)/cosh/

clean:
	rm *.chc

clean-all: clean
	rm -rf target
