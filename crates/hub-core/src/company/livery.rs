//! The company's own liveries (Luc: "vanuit het busbedrijf moet je ook een lakontwerp voor je
//! bus kunnen maken, wat natuurlijk geld kost"): a livery painted in the livery studio for a
//! bus model of the fleet becomes the company's design, and a bus of that model is painted in
//! it in the workshop.
//!
//! What it costs (Realistic, net, founding day's prices; Easy 0.6 times, Hard 1.3 times):
//! - *The design* (`design_fee`): a designer's corporate livery for a bus - the drawings, the
//!   colours, the proofs - €2,500. Saved again under its name (a change) a fifth of that.
//! - *The painting* (`paint_cost`): a foil wrap or a repaint of a 12 m bus in the company's
//!   livery €4,000 - a midibus three quarters of that, an articulated bus half again, a
//!   double-decker 1.4 times. Paid at once, and the bus wears the livery from then on, in the
//!   game too (`Vehicle::house_livery`) - Luc: "meteen toepassen". Every bus of the design's
//!   model can be painted in it in one go (`paint_all`), each at its painting's price.

use super::depot;
use super::model::{BookingKind, BusSize, Cents, Company, GameMode, Vehicle};
use serde::{Deserialize, Serialize};

/// A livery of the company's own: its name in the game (the livery studio's `.cti`), the bus
/// file it was painted for, when it was made, and what its design cost so far.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default)]
pub struct Design {
    pub name: String,
    pub bus: String,
    pub made: String,
    pub paid: Cents,
}

/// The design of a livery, and the painting of a 12 m bus (Realistic, founding day's prices).
pub const DESIGN_FEE: Cents = 2_500_00;
pub const PAINT_BASE: Cents = 4_000_00;

fn factor(d: GameMode) -> f64 {
    match d {
        GameMode::Casual => 0.6,
        GameMode::Normal => 1.0,
        GameMode::Realistic => 1.3,
    }
}

/// What painting a bus of a size costs against a solo bus.
pub fn size_factor(size: BusSize) -> f64 {
    match size {
        BusSize::Midi => 0.75,
        BusSize::Solo => 1.0,
        BusSize::Articulated => 1.5,
        BusSize::Double => 1.4,
    }
}

/// In whole euros.
fn euros(x: f64) -> Cents {
    ((x / 100.0).round() as Cents) * 100
}

/// What the design of a new livery costs the company now.
pub fn design_fee(c: &Company) -> Cents {
    euros(DESIGN_FEE as f64 * factor(c.game_mode) * c.price_index)
}

/// What painting a bus of the fleet in a livery of the company's costs.
pub fn paint_cost(c: &Company, v: &Vehicle) -> Cents {
    euros(PAINT_BASE as f64 * size_factor(v.kind.size) * factor(c.game_mode) * c.price_index)
}

fn same_file(a: &str, b: &str) -> bool {
    a.trim().replace('\\', "/").eq_ignore_ascii_case(&b.trim().replace('\\', "/"))
}

/// The company's design of that name (in any case).
pub fn design<'a>(c: &'a Company, name: &str) -> Option<&'a Design> {
    c.designs.iter().find(|d| d.name.trim().eq_ignore_ascii_case(name.trim()))
}

/// The company's designs for a bus file.
pub fn designs_for<'a>(c: &'a Company, bus: &str) -> Vec<&'a Design> {
    c.designs.iter().filter(|d| same_file(&d.bus, bus)).collect()
}

/// What saving a design under `name` costs: the design of a new livery, a fifth of it for one
/// the company has already (a change).
pub fn save_cost(c: &Company, name: &str) -> Cents {
    if design(c, name).is_some() {
        euros(design_fee(c) as f64 / 5.0)
    } else {
        design_fee(c)
    }
}

/// A livery painted in the studio for `bus` kept as the company's design: its cost booked.
/// Returns what was booked.
pub fn save_design(c: &mut Company, name: &str, bus: &str) -> Result<Cents, &'static str> {
    let name = name.trim();
    if name.is_empty() {
        return Err("The livery needs a name.");
    }
    let cost = save_cost(c, name);
    if !c.can_pay(cost) {
        return Err("Not enough cash.");
    }
    c.book(BookingKind::Livery, -cost, format!("Livery design \"{name}\""), false);
    let date = c.date.clone();
    match c.designs.iter_mut().find(|d| d.name.trim().eq_ignore_ascii_case(name)) {
        Some(d) => {
            d.paid += cost;
            d.bus = bus.to_string();
        }
        None => c.designs.push(Design { name: name.to_string(), bus: bus.to_string(), made: date, paid: cost }),
    }
    Ok(cost)
}

/// Whether a bus wears the company's livery `name` already.
pub fn wears(v: &Vehicle, name: &str) -> bool {
    v.house_livery.as_deref().is_some_and(|h| h.trim().eq_ignore_ascii_case(name.trim())) && v.livery.trim().eq_ignore_ascii_case(name.trim())
}

/// Paint a bus of the fleet in one of the company's designs, at once: its painting booked and
/// the bus in the livery from now on (a painting of it still waiting in the workshop dropped).
/// Returns what was booked.
pub fn paint(c: &mut Company, vehicle: u32, name: &str) -> Result<Cents, &'static str> {
    let Some(v) = c.vehicle(vehicle).cloned() else { return Err("This bus is not in the fleet.") };
    if !v.held_on(&c.date) {
        return Err("This bus is not in the fleet.");
    }
    let Some(d) = design(c, name).cloned() else { return Err("The company has no such livery.") };
    if !same_file(&d.bus, &v.bus) {
        return Err("This livery was made for another bus.");
    }
    if wears(&v, name) {
        return Err("The bus wears this livery already.");
    }
    let cost = paint_cost(c, &v);
    if !c.can_pay(cost) {
        return Err("Not enough cash.");
    }
    c.site.jobs.retain(|j| !(j.vehicle == vehicle && j.kind == depot::JobKind::Paint && j.started.is_none()));
    c.book(BookingKind::Livery, -cost, format!("{} {} ({})", v.number, v.name, depot::JobKind::Paint.label()), false);
    if let Some(x) = c.fleet.iter_mut().find(|x| x.id == vehicle) {
        x.livery = d.name.clone();
        x.house_livery = Some(d.name);
    }
    Ok(cost)
}

/// The buses of the fleet the design `name` is for (its bus model) that do not wear it yet.
pub fn unpainted(c: &Company, name: &str) -> Vec<u32> {
    let Some(d) = design(c, name) else { return Vec::new() };
    c.fleet.iter().filter(|v| v.held_on(&c.date) && same_file(&d.bus, &v.bus) && !wears(v, name)).map(|v| v.id).collect()
}

/// What painting all of them costs.
pub fn paint_all_cost(c: &Company, name: &str) -> Cents {
    unpainted(c, name).iter().filter_map(|id| c.vehicle(*id)).map(|v| paint_cost(c, v)).sum()
}

/// Paint every bus of the design's model in it at once (`unpainted`): all or none - refused
/// when the cash is not enough for all of them. Returns how many were painted and the cost.
pub fn paint_all(c: &mut Company, name: &str) -> Result<(usize, Cents), &'static str> {
    if design(c, name).is_none() {
        return Err("The company has no such livery.");
    }
    let buses = unpainted(c, name);
    if buses.is_empty() {
        return Err("Every bus of this type wears this livery already.");
    }
    if !c.can_pay(paint_all_cost(c, name)) {
        return Err("Not enough cash.");
    }
    let mut cost = 0;
    for id in &buses {
        cost += paint(c, *id, name)?;
    }
    Ok((buses.len(), cost))
}

#[cfg(test)]
mod tests {
    use super::super::market::{self, MarketBus, Payment};
    use super::super::{found, Founding};
    use super::*;
    use crate::company::model::{BusKind, Drive};

    fn company(d: GameMode) -> Company {
        let mut c = found(&Founding { name: "Lack".into(), game_mode: d, date: "2024-03-04".into(), ..Default::default() }, "Luc");
        c.cash += 1_000_000_00;
        let bus = MarketBus { file: "Vehicles/Citaro/Citaro.bus".into(), name: "Citaro".into(), kind: BusKind { size: BusSize::Solo, drive: Drive::Diesel }, ..Default::default() };
        market::buy_new(&mut c, &bus, Payment::Cash, "").unwrap();
        c
    }

    #[test]
    fn a_design_costs_its_fee_and_a_bus_its_painting() {
        let mut c = company(GameMode::Normal);
        assert_eq!(design_fee(&c), 2_500_00);
        assert_eq!(paint_cost(&c, &c.fleet[0]), 4_000_00);
        assert!(design_fee(&company(GameMode::Casual)) < design_fee(&c) && design_fee(&company(GameMode::Realistic)) > design_fee(&c));
        // saved: the fee booked, the design the company's
        let cash = c.cash;
        assert_eq!(save_design(&mut c, " Stadtbus Orange ", "Vehicles/Citaro/Citaro.bus"), Ok(2_500_00));
        assert_eq!(c.cash, cash - 2_500_00);
        assert!(c.ledger.last().is_some_and(|b| b.kind == BookingKind::Livery && !b.kind.is_capital()));
        assert_eq!(designs_for(&c, "vehicles\\citaro\\citaro.bus").len(), 1);
        // saved again under its name: a change, a fifth
        assert_eq!(save_cost(&c, "stadtbus orange"), 500_00);
        assert_eq!(save_design(&mut c, "Stadtbus Orange", "Vehicles/Citaro/Citaro.bus"), Ok(500_00));
        assert_eq!((c.designs.len(), c.designs[0].paid), (1, 3_000_00));
        assert_eq!(save_design(&mut c, "  ", "x"), Err("The livery needs a name."));
        // not without the money
        let mut poor = c.clone();
        poor.cash = 100_00;
        assert_eq!(save_design(&mut poor, "Neu", "x"), Err("Not enough cash."));
    }

    #[test]
    fn a_bus_is_painted_at_once() {
        let mut c = company(GameMode::Normal);
        let id = c.fleet[0].id;
        save_design(&mut c, "Stadtbus Orange", "Vehicles/Citaro/Citaro.bus").unwrap();
        save_design(&mut c, "Other", "Vehicles/MAN/Lion.bus").unwrap();
        assert_eq!(paint(&mut c, id, "Other"), Err("This livery was made for another bus."));
        assert_eq!(paint(&mut c, id, "Nothing"), Err("The company has no such livery."));
        // (a painting a company saved earlier still has waiting in the workshop is not needed any
        // more)
        depot::order_paint(&mut c, id, "Stadtbus Orange", 4_000_00).unwrap();
        let cash = c.cash;
        assert_eq!(paint(&mut c, id, "Stadtbus Orange"), Ok(4_000_00));
        // paid, and the bus in the livery now - not tomorrow, not in the workshop
        let v = c.vehicle(id).unwrap();
        assert_eq!((v.livery.as_str(), v.house_livery.as_deref()), ("Stadtbus Orange", Some("Stadtbus Orange")));
        assert_eq!(c.cash, cash - 4_000_00);
        assert!(c.ledger.last().is_some_and(|b| b.kind == BookingKind::Livery && b.amount == -4_000_00 && b.text.contains("Painting")));
        assert!(!v.in_workshop(&c.date) && c.site.jobs.is_empty());
        // painted: not again
        assert_eq!(paint(&mut c, id, "Stadtbus Orange"), Err("The bus wears this livery already."));
        // not without the money
        save_design(&mut c, "Blue", "Vehicles/Citaro/Citaro.bus").unwrap();
        c.cash = 100_00;
        assert_eq!(paint(&mut c, id, "Blue"), Err("Not enough cash."));
        assert_eq!(c.vehicle(id).unwrap().livery, "Stadtbus Orange");
    }

    #[test]
    fn every_bus_of_the_type_is_painted_in_one_go() {
        let mut c = company(GameMode::Normal);
        let citaro = c.fleet[0].clone();
        let lion = MarketBus { file: "Vehicles/MAN/Lion.bus".into(), name: "Lion".into(), kind: BusKind { size: BusSize::Solo, drive: Drive::Diesel }, ..Default::default() };
        let mut buy = |file: &str| {
            let bus = MarketBus { file: file.into(), name: "x".into(), kind: citaro.kind, ..Default::default() };
            market::buy_new(&mut c, &bus, Payment::Cash, "").unwrap();
        };
        buy("Vehicles/Citaro/Citaro.bus");
        buy("vehicles\\citaro\\citaro.bus");
        market::buy_new(&mut c, &lion, Payment::Cash, "").unwrap();
        save_design(&mut c, "Stadtbus Orange", "Vehicles/Citaro/Citaro.bus").unwrap();
        // the first painted already: the two others of its model are left, not the Lion
        paint(&mut c, citaro.id, "Stadtbus Orange").unwrap();
        assert_eq!(unpainted(&c, "Stadtbus Orange").len(), 2);
        assert_eq!(paint_all_cost(&c, "Stadtbus Orange"), 8_000_00);
        // all or none: not one painted without the money for both
        let mut poor = c.clone();
        poor.cash = 5_000_00;
        assert_eq!(paint_all(&mut poor, "Stadtbus Orange"), Err("Not enough cash."));
        assert_eq!(unpainted(&poor, "Stadtbus Orange").len(), 2);
        let cash = c.cash;
        assert_eq!(paint_all(&mut c, "Stadtbus Orange"), Ok((2, 8_000_00)));
        assert_eq!(c.cash, cash - 8_000_00);
        assert!(c.fleet.iter().filter(|v| same_file(&v.bus, "Vehicles/Citaro/Citaro.bus")).all(|v| wears(v, "Stadtbus Orange")));
        assert!(!wears(c.fleet.iter().find(|v| v.bus.contains("Lion")).unwrap(), "Stadtbus Orange"));
        assert_eq!(paint_all(&mut c, "Stadtbus Orange"), Err("Every bus of this type wears this livery already."));
        assert_eq!(paint_all(&mut c, "Nothing"), Err("The company has no such livery."));
    }
}
