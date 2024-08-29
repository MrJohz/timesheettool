use anyhow::Result;
use chrono::Utc;
use timesheettool::{db, records};

fn main() -> Result<()> {
    let mut conn = db::establish_connection()?;

    let mut recs = records::Records::new(&mut conn);
    recs.add_record("hello, world", Utc::now())?;
    for record in recs.list_records()? {
        println!("{record:?}");
    }
    Ok(())
}
