# Quote It

A simple Rust CLI quoting utility inspired by [Quote It](https://github.com/mattperls-code-apps/quote-it-app)

## Installation

Ensure `cargo` is installed

### Install with crates.io

`cargo install quote-it`

### Install manually

Clone this repo

Run `cargo install --path . ` 

## Usage

`quote-it --help` for instructions

### Create a quote

`quote-it "Woah Rust is amazing!"`

### Add an author and a timestamp

`quote-it "Javascript sucks" -a "Person with common sense" -t`

### List quotes

`quote-it list`

### List quotes by author

`quote-it list -a "Matt Perls"`


## Roadmap

- [ ] Export quotes
- [ ] Copy a quote to clipboard (either ID system or interactive mode)
