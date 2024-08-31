use clap::{Args, Parser, Subcommand, ValueEnum};

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
    /// Start a new time sheet record
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
    ///
    /// Aliases: start, record
    #[clap(aliases = &["start", "record"])]
    Go(Go),

    /// Stop the current record
    ///
    /// If any task is open at the given time, stop that task.  By default,
    /// the time used is the current time, but this can optionally be set
    /// using a flag.
    Stop(Stop),

    /// List all records
    ///
    /// By default, shows all records for the last week.  This can be changed
    /// using the --granularity flag (to change how records are groups) and
    /// the --since flag (to change how many records to show).
    #[clap(aliases = &["list", "list-records"])]
    Ls(ListRecords),
}

#[derive(Args, Debug)]
pub struct Go {
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

#[derive(Args, Debug)]
pub struct ListRecords {
    /// how long back to show records
    ///
    /// Results will be rounded to the beginning of the relevant period.
    /// For example, if since is "1 week", then all records from the start
    /// of the current week will be shown.  Similarly, an argument of
    /// "2 months" will show all records from the current and previous months.
    #[arg(short = 's', long, default_value = "1 week")]
    pub since: String,
    /// when to show records until
    ///
    /// Results will be rounded to the beginning of the relevant period.
    /// For example, if until is "1 week", then records will be shown until
    /// the start of the current week.  Similarly, an argument of "2 months"
    /// will show all records up until the beginning of the previous month.
    /// The keyword "now" will show results until the current time.
    #[arg(short = 'u', long, default_value = "now")]
    pub until: String,

    /// how to aggregate records
    ///
    /// By default, results will be aggregated automatically according to the
    /// number of records shown.  For example, if a month's worth of records are
    /// shown, then the records will be aggregated into individual weeks.
    /// Otherwise, results will be aggregated according to the unit of time given
    /// here.  Use "All" to show each individual record.
    #[arg(short = 'g', long, default_value = "auto")]
    pub granularity: Granularity,
}

#[derive(ValueEnum, Debug, Clone)]
pub enum Granularity {
    /// automatically aggregate records
    Auto,
    /// show all records
    All,
    /// show time spent on tasks per day
    Daily,
    /// Show time spent on tasks per week
    Weekly,
    /// Show time spent on tasks per month
    Monthly,
}
