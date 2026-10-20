//! A change made on the company's pages is written behind the page (`company::saver`): the
//! command does not wait for the company's file, and the file has the change once the app ends
//! right after it (`company::quit`) - an edit, the app closed at once, the company opened again.
//! The data folder is a temporary one (`OMSIHUB_DATA_DIR`).

use hub_api::{call, Quiet};
use hub_core::company::{self as co, store};
use serde_json::json;

#[test]
fn an_edit_is_on_file_when_the_app_ends_at_once() {
    let data = std::env::temp_dir().join(format!("hub-api-saves-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&data);
    std::env::set_var(hub_core::paths::DATA_DIR_ENV, &data);

    let driver = hub_api::launcher::driver();
    let c = co::found(&co::Founding { name: "Write Behind".into(), date: "2024-03-04".into(), map: "maps/Grundorf/global.cfg".into(), ..Default::default() }, &driver);
    store::save(&data, &c).unwrap();
    call("co_boot", &json!({}), &Quiet).unwrap();
    call("co_open", &json!({ "id": c.id }), &Quiet).unwrap();

    // an edit (RosterFlow's Layers › Colour by: a change of the company), the app ended at once
    call("rf_act", &json!({ "act": "colour_by", "by": "duty" }), &Quiet).unwrap();
    hub_api::company::quit();
    assert_eq!(store::load(&data, &c.id).unwrap().planning.flow.colour_by, "duty", "the edit is on file");

    // and opened again: as it was left
    call("co_reread", &json!({ "id": c.id }), &Quiet).unwrap();
    let open = call("co_company", &json!({}), &Quiet).unwrap();
    assert_eq!(open["planning"]["flow"]["colour_by"], "duty", "{}", open["planning"]["flow"]);
    let _ = std::fs::remove_dir_all(&data);
}
