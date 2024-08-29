use anyhow::Result;
use timesheettool::db;

fn main() -> Result<()> {
    let mut conn = db::establish_connection();
    dbg!(db::query_records(&mut conn)?);
    Ok(())
}
