fn main() {
    let json = r#"{"name":"Alice","age":30,"tags":["rust","json"]}"#;
    let value = json_autotune::parse(json).unwrap();
    println!("Parsed: {:#?}", value);
    println!("name: {:?}", value["name"].as_str());
    println!("age: {:?}", value["age"].as_u64());
}