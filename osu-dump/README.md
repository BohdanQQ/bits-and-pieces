# Osu! Dump

A WIP mini tool for extraction of publicly available Osu! user data. Currently it provides an automated way to
collect and summarize user plays data.

## Setup

    cargo build --release
    cd /your/target/dir
    
    osu-dump --help

## Usage

    An Osu! profile data extraction tool.
    
    Usage: osu-dump [OPTIONS] <USER_ID> <COMMAND>
    
    Commands:                                                               
    most-played  Lists Most Played Beatmaps                               
    help         Print this message or the help of the given subcommand(s)
    
    Arguments:                                                              
    <USER_ID>  User ID
    
    Options:                                                                
    -v, --verbose          Request verbose output                         
    -o, --output <OUTPUT>  Output type [possible values: json]
    -h, --help             Print help information
    -V, --version          Print version information

### Note
The `-o, --output <OUTPUT>` option is not yet implemented.

## TODO
* proper terminal output format
* more stats / information about user's plays
* actual JSON output
  * possibly other sane options