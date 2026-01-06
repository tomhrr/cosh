## nrtm

A library for working with RPSL NRTMv3 servers, which allow for
retrieving object updates incrementally over time.  There is no
formal specification for this protocol, but see e.g. [Near Real Time
Mirroring v3](https://docs.db.ripe.net/RIPE-Database-Mirror/Near-Real-Time-Mirroring-v3)
and [Mirroring with IRRd](https://irrd.readthedocs.io/en/stable/users/mirroring/).

### Usage


    $ nrtm import
    $ radb nrtm.sources; [name get; RADB =] first
    h(
	"available": .t
	"name":      RADB
	"last":      4370184
	"first":     0
    )
    $ RADB 4370183 4370184 radb nrtm.query
    v[gen (
	0: h(
	    "object": (
		0: (
		    0: route
		    1: 130.137.240.0/24
		)
                ...
            )
	    "type":   DEL
	    "serial": 4370183
        )
        ...
    )
    $

### Functions

 - `nrtm.servers`
    - Returns a hash that maps from short NRTM server name to a hash
      containing the sever host name, port number, and server type
      (either "ripe" or "irrd").

 - `nrtm.sources`
    - Takes a short NRTM server name and returns the NRTM sources
      available from that server, as a list of hashes.  Each hash
      contains the source name, the first and last serials available
      for retrieval, and a boolean indicating whether the source is
      available for use.

 - `nrtm.query`
    - Takes a source name, an initial serial, a final serial, and a
      short NRTM server name.  Returns a generator over the specified
      objects from that source, where each element of the generator
      contains an object (per `rpsl.parse`), a serial, and an
      operation tag (either "ADD" or "DEL").
