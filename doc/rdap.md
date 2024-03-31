## rdap

A library for working with Registration Data Access Protocol (RDAP)
queries.  See the following RFCs for background:

 - [RFC 7480: HTTP Usage in the Registration Data Access Protocol (RDAP)](https://datatracker.ietf.org/doc/html/rfc7480)
 - [RFC 9082: Registration Data Access Protocol (RDAP) Query Format)](https://datatracker.ietf.org/doc/html/rfc9082)
 - [RFC 9083: JSON Responses for the Registration Data Access Protocol)](https://datatracker.ietf.org/doc/html/rfc9083)
 - [RFC 9224: Finding the Authoritative Registration Data (RDAP) Service](https://datatracker.ietf.org/doc/html/rfc9224)
 - [RFC 8521: Registration Data Access Protocol (RDAP) Object Tagging](https://datatracker.ietf.org/doc/html/rfc8521)

The bulk of the logic here is for looking up the server for a
particular query.

### Usage

    $ rdap import
    $ rdap.init
    $ 1.0.0.0/24 rdap
    h(
        "cidr0_cidrs":     (
            0: h(
                "length":   24
                "v4prefix": 1.0.0.0
            )
        )
        "country":         AU
        "endAddress":      1.0.0.255
	...
    )
    $

### Functions

 - `rdap.init`
    - Downloads the bootstrap files from the IANA server and stores
      them locally for later server lookup operations.

 - `rdap.refetch`
    - Downloads any bootstrap file that has been updated since it was
      last downloaded.  This should be run periodically via a
      cronjob or similar.

 - `rdap`
    - Takes a single IP address/range, ASN, domain name, or entity
      handle, and returns the RDAP result for the associated lookup
      operation as a hash.  If no server can be found for the
      argument, returns null.  (Entity handle server lookup is by way
      of object tags, and the coverage for that is very limited: most
      entities have to be retrieved by way of links from other
      objects, or server-specific fetch operations.)
