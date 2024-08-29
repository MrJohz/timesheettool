use anyhow::Result;
use chrono::Utc;
use timesheettool::{db, records};

fn main() -> Result<()> {
    let mut conn = db::establish_connection();
    dbg!(db::query_records(&mut conn)?);
    let recs = records::Records::new(&mut conn);
    dbg!(recs.add_record("hello, world", Utc::now())?);
    Ok(())
}
