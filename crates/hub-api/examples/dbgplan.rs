//! Why a company's day has no tours: the company, the day's timetable, and what each step of
//! `tours_of_day` makes of them.  cargo run -p hub-api --example dbgplan -- <company id> <date>
use hub_core::company as co;

fn main() {
    let id = std::env::args().nth(1).expect("company id");
    let date = std::env::args().nth(2).expect("date");
    let data = hub_core::paths::data_dir();
    let c = co::store::load(&data, &id).expect("company");
    println!("company {} map {} date {} game_mode {:?} autopilot {}", c.id, c.map, c.date, c.game_mode, co::autopilot::on(&c));
    for l in &c.lines {
        println!("  line {} number {} own {} service_from {:?}", l.name, l.number, l.own, l.service_from);
    }
    let lines = hub_core::list_lines(&c.map, &date).expect("lines");
    for cl in &c.lines {
        match lines.iter().find(|l| l.name.eq_ignore_ascii_case(&cl.name)) {
            None => println!("  {}: not in the day's timetable ({} lines)", cl.name, lines.len()),
            Some(l) => println!("  {}: {} tours, running {}", cl.name, l.tours.len(), l.tours.iter().filter(|t| t.runs).count()),
        }
    }
    let raw = co::network::automatic_tours_of_day(&c, &lines, &date);
    println!("automatic tours: {}", raw.len());
    let tours = co::network::tours_of_day(&c, &lines, &date);
    println!("tours of day: {}", tours.len());
    for t in tours.iter().take(8) {
        println!("  {} / {} trips {} unplanned {}", t.line, t.tour, t.trips.len(), t.unplanned);
    }
    let p = co::plan::day_plan(&c, &date, tours, &[], &[], false);
    println!("day plan tours: {}", p.tours.len());
    for t in p.tours.iter().take(8) {
        println!("  {} / {} bus {:?} by_player {} duties {}", t.tour.line, t.tour.tour, t.bus, t.by_player, t.duties.len());
    }
}
