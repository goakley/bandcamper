<img style="height:64px;" height=64 src="LOGO.svg" alt="bandcamper logo" />

# Bandcamper

<a href="https://github.com/goakley/bandcamper/actions?query=branch%3Amain"><img src="https://img.shields.io/github/workflow/status/goakley/bandcamper/CI/main" /></a>
<img src="https://img.shields.io/github/license/goakley/bandcamper" />

Bandcamper is a tool for syncing your [Bandcamp](https://bandcamp.com/) music collection to your computer.

<img style="height:300px;" height=300 src="image.png" alt="example of a downloaded bandcamp collection" />

## Usage

Either download a [pre-built version of the tool](https://github.com/goakley/bandcamper/releases/) or build the executable yourself (see below).

The tool is run through the command line interface.
The simplest way to run the tool is to give it the folder in which you want to save your music:

* On Windows: `bandcamper %USERPROFILE%\Music\Bandcamp`
* On Mac / Linux: `bandcamper ~/Music/Bandcamp`

Run `bandcamper --help` for details on alternate invocations.

## Building

This tool is a standard Rust Cargo project, and can be built and run as such:

```
git clone git@github.com:goakley/bandcamper.git
cd bandcamper
cargo build
./target/debug/bandcamp ...
```
