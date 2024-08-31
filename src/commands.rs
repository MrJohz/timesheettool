use clap::{Args, Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Arguments {
    /// increase the verbosity
    ///
    /// This flag can be used multiple times to increase the amount of information
    /// produced by timesheettool
    #[arg(global = true, short, long, action = clap::ArgAction::Count, help_heading = "Logging")]
    pub verbose: u8,

    /// output no logging
    ///
    /// Setting quiet disables all logging to stderr.  Data will only be printed
    /// to stdout, and only for commands that output information as their main
    /// action.
    #[arg(global = true, long, action = clap::ArgAction::SetTrue, help_heading = "Logging")]
    pub quiet: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Create a new time sheet record
    ///
    /// Creates a new time sheet record with the given task name.
    /// By default, this will be an active record (i.e. no end date),
    /// starting from the current time.  The user can optionally set
    /// a start time and an end time using the flags provided.
    ///
    /// If an open record exists in the database that would overlap with
    /// this record, then that record will be ended with the start time of
    /// the newly created record.  This can be disabled using the
    /// --allow-overlap flag.
    #[clap(aliases = &["start", "go"])]
    Log(Log),

    /// Stop the current record
    ///
    /// If any task is open at the given time, stop that task.  By default.
    /// the time used is the current time, but this can optionally be set
    /// using a flag.
    Stop(Stop),
}

#[derive(Args, Debug)]
pub struct Log {
    /// task name
    ///
    /// Provides the task name that this record should be logged under.  If
    /// the tag name doesn't exist yet in the database, it will be created.
    pub name: String,

    /// record start time
    ///
    /// Defaults to the current time.  Can be specified as a ISO-8601-style
    /// string, or as a relative string.  (See documentation for the exact
    /// format of this string.)
    #[arg(short = 's', long)]
    pub start: Option<String>,

    /// record end time
    ///
    /// Defaults to no end time if not set (i.e. the task is marked as still in progress).
    /// Can be specified as a ISO-8601-style string, or as a relative string.  (See
    /// documentation for the exact format of this string.)
    #[arg(short = 'e', long)]
    pub end: Option<String>,

    /// allow this record to overlap other records in the database
    #[arg(long, action=clap::ArgAction::SetTrue)]
    pub allow_overlap: bool,
}

#[derive(Args, Debug)]
pub struct Stop {
    /// record end time
    ///
    /// Defaults to the current time.  Can be specified as a ISO-8601-style
    /// string, or as a relative string.  (See documentation for the exact
    /// format of this string.)
    #[arg(short = 'e', long)]
    pub end: Option<String>,
}
