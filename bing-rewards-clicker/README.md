# Bing search automation

A simple script to try out Playwright.

## Disclaimer

Automation of searches is against *certain* ToS...

## Setup

### Download a word list
    curl https://gist.githubusercontent.com/dstrelau/1005478/raw/cdb9b07dd2d0a0bfbcad79731dd07d097aab23b3/wordlist.txt -o ./wordlist.txt

### Windows:
    py -m venv ./.venv
    ./.venv/Scripts/activate 
    pip install -r ./requirements.txt
    playwright install

### Linux
    python3 -m venv ./.venv
    ./.venv/bin/activate
    pip3 install -r ./requirements.txt
    playwright install

## Run
    pytest .\bing.py --headed -s
