# Osu! Dump

A WIP mini tool for extraction of publicly available Osu! user data. Currently it provides an automated way to
collect and summarize user plays data.

## Setup

    cargo build --release
    cd /your/target/dir
    
    osu-dump --help

## Usage

    An Osu! profile data extraction tool.

    Usage: osu-dump.exe [OPTIONS] <USER_ID> <COMMAND>

    Commands:
    most-played  Lists Most Played Beatmaps. Data collection may take very long time depending on the amount of beatmaps played
    help         Print this message or the help of the given subcommand(s)

    Arguments:
    <USER_ID>  User ID

    Options:
    -v, --verbose
            Request verbose output
    -o, --output <OUTPUT>
            Output type [possible values: json]
    -u, --api-url-base <API_URL_BASE>
            API URL to use. It is recommended to setup a local caching proxy to save time and bandwidth [default: https://osu.ppy.sh]
    -r, --request-limit <REQUEST_LIMIT>
            API request limit in the format "N:M" where N is the maximum number of calls in M seconds. Use 0 to represent "no limits" [default: 50:60]
    -h, --help
            Print help information
    -V, --version
            Print version information

### `most-played` command

    Lists Most Played Beatmaps. Data collection may take very long time depending on the amount of beatmaps played
    
    Usage: osu-dump <USER_ID> most-played [OPTIONS] [MODES]...
    
    Arguments:
    [MODES]...  Osu beatmap modes to filter. Accepts all by default. [possible values: standard, mania, taiko, catch]
    
    Options:
    -l, --limit <LIMIT>  Limits the amount of beatmaps outputted. LIMITS ACCURACY because not all data will be fetched from the Osu! API
    -h, --help           Print help information

## TODO
* proper terminal output format
* more stats / information about user's plays
* possibly other sane output options
* parallelize API requests, crunch data on the fly