RUSTC ?= rustc -C opt-level 3

PREFIX ?= /usr
DESTDIR ?=

# ------------------ RUST BINARIES -----------------------
CANTALLIB = libcantal.rlib
ARGPARSELIB = rust-argparse/libargparse.rlib

all: bin js

bin: libcantal.rlib cantal cantal_agent

test: cantal_test
	./cantal_test

libcantal.rlib: $(ARGPARSELIB) src/query/lib.rs src/query/*.rs
	$(RUSTC) src/query/lib.rs -g -o $@ \
		-L rust-argparse -L .

cantal_agent: $(ARGPARSELIB) libcantal.rlib src/agent/main.rs src/agent/*.rs
cantal_agent: src/agent/*/*.rs
	$(RUSTC) src/agent/main.rs -g -o $@ \
		-L rust-argparse -L .

cantal: $(ARGPARSELIB) libcantal.rlib src/cli/main.rs src/cli/*.rs
	$(RUSTC) src/cli/main.rs -g -o $@ \
		-L rust-argparse -L .

$(ARGPARSELIB):
	make -C rust-argparse libargparse.rlib

# ------------------ JAVASCRIPTS -----------------------

js:
	-mkdir public/js 2>/dev/null
	make -C frontend

# ------------------ INSTALL -----------------------

install:
	install -d $(DESTDIR)$(PREFIX)/bin
	install -d $(DESTDIR)$(PREFIX)/lib/cantal
	install -m 755 cantal $(DESTDIR)$(PREFIX)/bin/cantal

	install -m 755 cantal-agent $(DESTDIR)$(PREFIX)/lib/cantal/cantal-agent
	ln -s ../lib/cantal/cantal-agent $(DESTDIR)$(PREFIX)/bin/cantal-agent
	# setcap is required to be able to read other processes environment
	# without root privileges
	setcap "cap_sys_ptrace=ep cap_dac_read_search=ep" cantal-agent

	cp -r public $(DESTDIR)$(PREFIX)/lib/cantal/


.PHONY: all install test bin js webpack
.DELETE_ON_ERROR:
