## rpsl

A library for working with Routing Policy Specification Language
(RPSL) objects.  See [RFC 2622: Routing Policy Specification Language
(RPSL)](https://datatracker.ietf.org/doc/html/rfc2622) for background.

### Usage

    $ rpsl import
    $ 193.0.11.51 ripe rpsl.query
    v[gen (
        0: v[gen (
            0: (
                0: inetnum
                1: v[ip 193.0.10.0-193.0.11.255]
            )
            1: (
                0: netname
                1: RIPE-NCC
            )
            2: (
                0: descr
                1: "RIPE Network Coordination Centre"
            )
            ...
        )]
    )]
    $

### Functions

 - `rpsl.query`
    - Takes an RPSL server query string and an RPSL server name or
      address as its arguments.  Sends the query to the server and
      returns a generator over the parsed objects from the response.
      Short names may be used for the RPSL server name, with the
      mappings being per `rpsl.servers`.

 - `rpsl.servers`
    - Returns a hash that maps from short RPSL server name to the
      actual domain name used for queries.

 - `rpsl.query-raw`
    - Operates in the same way as `rpsl.query`, though without
      converting any of the object fields from text to native types.

 - `rpsl.parse`
    - Parse a single RPSL object from a generator over RPSL text
      content.

 - `rpsl.parsem`
    - Parse multiple RPSL objects from a generator over RPSL text
      content.

 - `rpsl.str`
    - Takes a list of [key, value] pairs (as returned by `rpsl.parse`)
      and converts it back to RPSL text format. This performs the
      reverse operation of `rpsl.parse`.
