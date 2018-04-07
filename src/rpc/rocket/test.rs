use instance::*;
use rocket::http::ContentType;
use rocket::local::Client;

#[test]
fn born() {
    let rocket = super::start_rocket_server();
    let client = Client::new(rocket).expect("valid rocket instance");
    let json = ::serde_json::to_string(&(
        Instance::default())).unwrap();
    let mut response = client.post("/input")
        .body(json)
        .header(ContentType::JSON)
        .dispatch();

    let rtn = response.body_string().unwrap();
    println!("{:?}", rtn);
    assert_eq!(rtn, r#"{"Ok":[11,172,237,228,64,20,63,157,183,32,23,63,104,161,201,51]}"#);
}