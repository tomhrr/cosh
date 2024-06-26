## rpkiv

A library that acts as a wrapper around the commonly-used RPKI
validators.

### Usage

    $ rpkiv import
    $ h(tals (/usr/local/etc/rpki/apnic.tal)
        name apnic
        type rpki-client
        exec /usr/local/sbin/rpki-client) rpkiv.init;
    $ rpkiv.run;
    $ apnic rpkiv.vrps; shift;
    (
        0: 13335
        1: v[ip 1.0.0.0/24]
        2: 24
        3: apnic
        4: v[datetime 2024-04-15 14:17:53 UTC]
    )
    $ 1.0.0.0/25 13335 apnic rpkiv.rov
    valid
    $ apnic rpkiv.cd; cache/rpki.apnic.net/member_repository/A91872ED/ED8C96901D6C11E28A38A3AD08B02CD2/797B4DEC293B11E8B187196DC4F9AE02.roa apnic rpkiv.file
    h(
        "aia":          rsync://rpki.apnic.net/repository/B527EF581D6611E2BB468F7C72FD1FF2/aPr52s4ZdoysPU7XuyQ3K_-m0Bg.cer
        "aki":          68:FA:F9:DA:CE:19:76:8C:AC:3D:4E:D7:BB:24:37:2B:FF:A6:D0:18
        "cert_issuer":  /CN=A91872ED/serialNumber=68FAF9DACE19768CAC3D4ED7BB24372BFFA6D018
        "cert_serial":  2E61
        "expires":      v[datetime 2024-04-15 14:17:53 UTC]
        "file":         cache/rpki.apnic.net/member_repository/A91872ED/ED8C96901D6C11E28A38A3AD08B02CD2/797B4DEC293B11E8B187196DC4F9AE02.roa
        "hash_id":      m0T5p+j/9s+rm9D6YdSh//2Alx2Eae+Du8lbaB1hR7Y=
        "sia":          rsync://rpki.apnic.net/member_repository/A91872ED/ED8C96901D6C11E28A38A3AD08B02CD2/797B4DEC293B11E8B187196DC4F9AE02.roa
        "signing_time": v[datetime 2021-02-11 14:20:11 UTC]
        "ski":          02:1A:B3:B2:53:BA:78:7F:19:6D:41:F2:65:D6:2E:A0:44:FA:C0:A3
        "type":         roa
        "valid_since":  v[datetime 2021-02-11 14:20:11 UTC]
        "valid_until":  v[datetime 2031-03-31 00:00:00 UTC]
        "validation":   OK
        "vrps":         v[gen (
            0: h(
                "asid":   13335
                "maxlen": 24
                "prefix": v[ip 1.0.0.0/24]
            )
            1: h(
                "asid":   13335
                "maxlen": 24
                "prefix": v[ip 1.1.1.0/24]
            )
        )]
    )

### Functions

 - `rpkiv.init`
    - Takes a hash containing the following entries as its single
      argument (each is required):
        - `name`: the name of the new instance
        - `tals`: a list of TAL paths for the new instance
        - `type`: a validator type (`rpki-client`, `routinator`, or `fort`)
        - `exec`: the path to the validator executable
      Initialises a new RPKI validator instance.

 - `rpkiv.instances`
    - Returns a generator over all of the current instance names.

 - `rpkiv.clear`
    - Takes an instance name, and deletes all data associated with
      that instance.

 - `rpkiv.cd`
    - Takes an instance name, and changes directory to that instance
      (for inspecting cache files and similar).

 - `rpkiv.run`
    - Takes an instance name, and runs RPKI validation for the
      instance.

 - `rpkiv.last-stdout`
    - Takes an instance name, and returns the standard output for the
      last validation run for the instance.

 - `rpkiv.last-stderr`
    - Takes an instance name, and returns the standard error for the
      last validation run for the instance.

 - `rpkiv.vrps`
    - Takes an instance name, and returns the VRPs for the instance as
      a list, where each list element is a list containing the origin
      ASN, prefix, and max-length.  For `routinator` and
      `rpki-client`, the TA is also included.  For `rpki-client`, the
      effective date of expiry is also included.

 - `rpkiv.rov`
    - Takes a prefix, an origin ASN, and an instance name as its
      arguments, and returns the origin validation status for that
      announcement ('valid', 'invalid', or 'unknown').

### `rpki-client`-specific functions

 - `rpkiv.file`
    - Takes a cache path and an instance name as its arguments, and
      returns data for the specified cache path per the `-f` option to
      `rpki-client` (with field values converted to native data types
      where possible).

 - `rpkiv.files`
    - Takes a list of cache paths and an instance name as its
      arguments, and returns a generator over the data for the
      specified cache paths per the `-f` option to `rpki-client` (with
      field values converted to native data types where possible).
      (This function calls `rpki-client` with multiple paths each
      time, which means that it's faster than `rpkiv.file`.)
