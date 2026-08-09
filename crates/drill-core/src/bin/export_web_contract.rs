fn main() {
    let contract = drill_core::web_contract();
    println!(
        "{}",
        serde_json::to_string_pretty(&contract).expect("web contract must serialize")
    );
}
