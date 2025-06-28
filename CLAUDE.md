# Development Guidelines

This document contains critical information about working with this
codebase. Follow these guidelines precisely.

## Build

 - To build the project: `make`.
 - To run tests: `make test`.
 - To install: `make install`.

## Tests

 - New features require tests.
 - Bug fixes require regression tests.
 - Tests (per `make test`) must pass before changes are given to the
   user.

## Code quality

 - Line length: 80 chars maximum.
 - Comments must be full sentences, and end with a period.
