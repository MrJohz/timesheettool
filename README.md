# TimeSheetTool (`tst`)

A little tool I use to track the hours I've been working, and the projects I've been working on. This probably isn't fit to use, and this README mainly exists so that I can remember what I was thinking last time I made changes in this project.

## Usage

Run `tst --help` for more documentation on the available commands, and how those commands work. However, in general, there are four main commands:

```bash
# Create a new record, starting today at hh:mm and ending today at hh:mm.
# If `--start` is not provided, then default to the current time.
# If `--end` is not provided, then the record will be left open as an ongoing record.
tst go [--start hh:mm] [--end hh:mm] <project> "task description"

# Stops an existing record at the time provided.  If `--end` is not provided,
# then the end time defaults to the current time.
tst stop [--end hh:mm]

# Updates an existing record with new data.  The record ID can be
# found using `tst ls`.
tst edit <record_id> [--start hh:mm] [--end hh:mm] [--project project] [--task task]

# Lists existing records.  By default, show all records from the current week,
# use `--since` to change this.  Longer time periods will be shown in a more
# compacted format (e.g. all records, then daily records, then weekly records,
# etc), use `--granularity` to change this.
tst ls [OPTIONS]
```

Note that timestamps (shown above as `hh:mm`) can be written in two ways:

- as an `hh:mm` 24-hour format (e.g. `16:40`), in which case the date is assumed to be the current date
- as a standard ISO format, in which case the date is taken from the timestamp.

When listing hours with a granularity of `daily` or coarser, hours in the same project will be summed together, and the number of hours in that project will be rounded to the next-largest quarter-hour. This can be configured in the config file.

## Installing

Currently, the only installation method I'm using is cloning the project and running `cargo install --path .`

```bash
git clone git@github.com:MrJohz/timesheettool.git
cd timesheettool
cargo install --path .
```

This will install the release version of the project into the Cargo bin folder, which should be in your path. (This may depend on how you've set up Cargo/rustup).

## Development

Use Cargo for development. There are a bunch of unit tests in various parts of the system (of varying quality), which can be run using `cargo test`. The standard Rust formatter is used for formatting, and `cargo clippy` is used for linting. None of this is currently checked in CI.

You may find it useful to copy the `example.timesheettool.toml` file to `timesheettool.toml` in the project directory (this file is ignored by git), and use that for local development.

## Licensing, Etc

This code is licensed under the Mozilla Public License 2.0 (MPL-2.0). See the license file provided in the project.

## TODOs (Assorted)

- Granularities other than `all` and `daily` aren't implemented. They should be implemented in a way that's useful for me.
- `tst overtime` exists as a way of tracking how many hours I've worked compared to my expected 8hr day, but needs more configuration options and a better way of viewing the time.
