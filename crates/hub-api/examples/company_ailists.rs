//! A company's AI list for the game written or taken away by hand (`hub_core::company::ailist`,
//! which a company's duty writes itself): for trying a company's day in the game.
//!
//!   cargo run -p hub-api --example company_ailists -- <company id> <date>
//!   cargo run -p hub-api --example company_ailists -- off
use hub_core::company as co;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let content = hub_core::paths::content_dir().expect("openOMSI's content folder");
    if args.get(1).map(String::as_str) == Some("off") {
        println!("removed {}", co::ailist::remove_all(&content));
        return;
    }
    let (id, date) = (&args[1], &args[2]);
    let c = co::store::load(&hub_core::paths::data_dir(), id).expect("the company");
    let lines = hub_core::list_lines(&c.map, date).expect("the day's timetable");
    let tours = co::network::tours_of_day(&c, &lines, date);
    let plan = co::plan::day_plan(&c, date, tours, &[], &[], false).to_plan();
    let vfs = hub_core::paths::vfs().expect("the game's folders");
    let f = co::ailist::write(&c, &plan.tours, date, &vfs, &content).expect("written");
    println!("{} company tours on their own buses, {} other bus tours out, lines off: {:?}", f.tours, f.taken_out, f.off);
    println!("in {}", co::ailist::folder_of(&content, &c.map).unwrap().display());
}
