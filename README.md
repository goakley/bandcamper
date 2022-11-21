<img style="height:128px;" height=128 src="LOGO.svg" alt="bandcamper logo" />

# Bandcamper

Bandcamper is a tool for syncing your [Bandcamp](https://bandcamp.com/) music collection to your computer.

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
