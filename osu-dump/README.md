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
    -a, --api-url-base <API_URL_BASE>
    API URL to use. It is recommended to setup a local caching proxy to save time and bandwidth [default: https://osu.ppy.sh]
    -h, --help
    Print help information
    -V, --version
    Print version information

## TODO
* proper terminal output format
* more stats / information about user's plays
* possibly other sane output options
* parametrize api limiting (with OK default for normal API)
* parallelize API requests, crunch data on the fly