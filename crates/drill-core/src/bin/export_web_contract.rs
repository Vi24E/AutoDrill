fn main() -> Result<(), Box<dyn std::error::Error>> {
    let contract = drill_core::web_contract()?;
    println!("{}", serde_json::to_string_pretty(&contract)?);
    Ok(())
}
