# A simple script to create HTML from an SVD file

This is a simple script written in Rust language to create a single HTML file from an SVD file.

![How it looks like](https://raw.githubusercontent.com/bjoernQ/svd2html/main/docs/example.png "How it looks like")

It's really simple and makes some assumptions (e.g. registers are 32 bit wide, the target is little endian etc.)

I created this for myself to make it easier to visually compare and inspect the contents of SVD files. But maybe it's useful to you.

It also omits a lot of things which I might or might not add in the future as needed by myself.

Also there is almost no error handling in place, currently. i.e. if an error occurs you won't get a nice error message but just some gibberish.

## Installation

```
cargo install --path .
```

## How to use

```text
svd2html 0.1.0

USAGE:
    svd2html [OPTIONS] --input <INPUT>

OPTIONS:
    -h, --help               Print help information
    -i, --input <INPUT>      SVD file to parse
    -o, --output <OUTPUT>    Directory to write generated HTML files [default: output]
    -V, --version            Print version information
```
