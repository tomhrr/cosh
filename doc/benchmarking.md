## cosh

### Benchmarking

There are a number of basic benchmarking tests and associated programs
in the `bm` directory, for ensuring that changes don't have unintended
or problematic effects on performance.  Benchmarking results are
stored in the `bm/results` directory.

#### Programs

 * `run-bms`: run all current benchmark programs and print the
   results to standard output in the format described below.
 * `cmp-bms`: takes two result file paths as its arguments, and prints
   human-readable output about the differences to standard output.

#### Results format

    {
      "programs": {
        "num-subtract": {
          "cosh": {
            "avg_mem":  # The number of allocations performed by the program.
            "avg_inst": # The number of instructions executed by the program.
            "avg_time": # The time taken to run the program (in milliseconds).
            "max_rss":  # The maximum resident set size during execution
                        # (in kilobytes).
          }
        },
        ...
      },
      "other": {
        "time": # The time at which the benchmarks were taken.
      },
      "cosh": {
        "hash":   # A hash over the source.
        "commit": # The current Git commit hash.
      }
    }

#### Notes

It may be that `avg_time` should drop out of the benchmark results,
given that it won't be stable across different machines.

Each benchmark program also has a corresponding Perl program, which
can be used for basic performance comparisons with Perl.
