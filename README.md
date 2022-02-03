# A simple script to create HTML from an SVD file

This is a simple script written in Rust language to create a single HTML file from an SVD file.

![How it looks like](https://raw.githubusercontent.com/bjoernQ/svd2html/main/docs/example.png "How it looks like")

It's really simple and makes some assumptions (e.g. registers are 32 bit wide, the target is little endian etc.)

I created this for myself to make it easier to visually compare and inspect the contents of SVD files. But maybe it's useful to you.

It also omits a lot of things which I might or might not add in the future as needed by myself.

Please note that it will generate a huge HTML file which might bring your browser down on it's knees.
Speaking of browsers: I only tested it with Chrome and I am really bad at HTML and CSS. So it might not look good in other browsers.

Also there is almost no error handling in place, currently. i.e. if an error occurs you won't get a nice error message but just some gibberish.

## Installation

```
cargo install --path .
```

## How to use

```text
svd2html 

USAGE:
    svd2html --input <INPUT> --output <OUTPUT>

OPTIONS:
    -h, --help               Print help information
    -i, --input <INPUT>      
    -o, --output <OUTPUT>    
```
