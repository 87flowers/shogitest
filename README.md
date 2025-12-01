# Shogitest

Shogitest is a command-line tool for running shogi engine tournaments, and for shogi engine testing.
Its command line interface is designed to be very familiar to users of [Fastchess](<https://github.com/Disservin/fastchess>).

You can configure shogi-specific time controls for engines, execute matches concurrently, calculate
pentanomial statistics, and test new patches for engines with the generalized sequential probability
ratio test.

## Getting started

Building and installing Shogitest from source is the same as any other Rust application:

1. `git clone https://github.com/87flowers/shogitest`
2. `cd shogitest`
3. `cargo install --path .`

This will add `shogitest` to your `~/.cargo/bin/` directory.

A Makefile is in the repository, but this is intented for use in automated building by distributed
testing frameworks like [OpenBench](<https://github.com/AndyGrant/OpenBench>).

## Example usage

You must provide an opening book, and at least two engines.

### 400 game tournament

```bash
shogitest.exe \
    -engine cmd=engine1.exe -engine cmd=engine2.exe \
    -each tc=10+0.1 -rounds 200 -concurrency 4 \
    -openings file=4moves_shogi.sfen.epd
```

### SPRT

```bash
shogitest.exe \
    -engine cmd=engine-dev.exe \
    -engine cmd=engine-base.exe \
    -each tc=10+0.1 \
    -concurrency 4 \
    -openings file=4moves_shogi.sfen.epd order=random \
    -sprt elo0=0 elo1=5 alpha=0.05 beta=0.05
```

- Specifying `-rounds` is not required, as infinite is the default option.
- Specifying `-repeat` is not required, as this is the default option (shogitest doesn't allow you to not have game pairs).

## Command line options

### Tournament settings

- `-concurrency N`

    Play N games concurrently. Default value is `1`.

- `-rounds N`

    Play N rounds. All games within the round use the same opening. If left unspecified, the default value is infinite. Must be non-zero.

- `-sprt elo0=ELO0 elo1=ELO1 alpha=ALPHA beta=BETA`

    Set parameters for a generalised sequential probability ratio test (GSPRT).

  - Elo are specified in normalized elo (nElo), for each of the hypotheses under test.
  - `alpha` is the desired false positive rate, and `beta` is the desired false negative rate.
  - We recommend using `-sprt` with an infinite number of rounds, as the GSPRT will automatically terminate when confidence thresholds are reached.
  - `-sprt` is only valid when exactly two `-engine`s are specified.

- `-games N`

    Play N games within each round. Must be a non-zero multiple of two. Default value is `2`. All games within a round use the same opening.
    This is provided mainly for compatibility as any value other than two is not recommended.

- `-repeat`

    This is equivalent to `-games 2`. Provided mainly for compatibility, as this the default.

- `-variant standard`

    This is provided for compatibility with external tooling. The only valid value for variant is `standard`.

### Engine configuration

- `-each OPTIONS*`

    Apply the options list to all engines. See below for a list of possible options.

- `-engine OPTIONS*`

    Declare an engine with the specified configuration specified by the options list.

  - `name=NAME`: Overwrite the default name detection (which looks at UCI `id name`).
  - `cmd=CMD`: Specify engine executable.
  - `dir=DIR`: Specify engine working directory.
  - `proto=usi`: Specify the engine protocol. Only `usi` is supported.
  - `tc=MIN:SEC+INC`: Specify Fischer time control.
  - `tc=MIN:SEC,BYOYOMI`: Specify Byoyomi time control.
  - `tc=movetime=SEC`: Specify movetime time control.
  - `tc=N=NODES`: Specify node count time control. (e.g. `tc=N=5000`)
  - `st=SEC`: Compatibility alias for `tc=movetime=SEC`
  - `nodes=NODES`: Compatibility alias for `tc=N=NODES`
  - `option.NAME=VALUE`: Set engine-specific USI options.
  - `timemargin=MILLISECS`: Set time margin for exceeding time limit.
  - `restart=(on|off)`: Restart engine in between games, defaults to `off`.

You can only specify one time control. Multiple time controls do not stack.

### Adjudication

- `-maxmoves N`

    Adjudicate a draw if the game reaches N moves. Defaults to `512`. You can specify `inf` to lift this limit.

- `-draw movenumber=N movecount=N score=N`

    Enables draw adjudication.

  - `movenumber`: Number of ply before checking for a draw. Opening book ply are not considered here.
  - `movecount`: Number of consecutive moves (both sides) that need to be below the score threshold.
  - `score`: Score threshold in cp.

- `-resign movecount=N score=N [twosided=(false|true)]`

    Enables resign adjudication.

  - `movecount`: Number of consecutive moves that need to be above the score threshold.
  - `score`: Score threshold in cp.
  - `twosided`: Determines if the consecutive moves are from both sides or just one side. Defaults to `false` (one-sided).

### Opening Book

An opening book is required.

- `-openings file=NAME [format=epd] [order=(sequential|random)] [start=N]`

  - `file=NAME`: Specifies the location of the openings file
  - `format=epd`: Optional. Only valid format. File is a list of sfens.
  - `order=(sequential|random)`: Specifies whether we shuffle openings. Defaults to `sequential`.
  - `start=N`: Specifies the starting index of the opening book. This is one-indexed. Default is `1`.

- `-srand SEED`

    Specify the seed for opening book shuffling.

### Output

- `-ratinginterval N`

    Set a interval for rating reports. Default value is `10`. Specifying `0` turns off interval reporting.

- `-pgnout file=FILE [nodes=(true|false)] [seldepth=(true|false)] [nps=(true|false)] [hashfull=(true|false)] [timeleft=(true|false)] [latency=(true|false)]`

    Output games in a pseudo-PGN format with optional tracking of other statistics. Default for all tracking options is `false`.
    This is primarily intended for OpenBench compatibility.

- `-event NAME`

    Set event name for PGN header.

- `-site NAME`

    Set site name for PGN header.

- `-testEnv`

    Adjust output for running Shogitest in a test environment such as OpenBench.
