#[tokio::test]
async fn ascii_to_felt() {
    let s = "Hello";
    for b in s.bytes() {
        println!("{}", b); // ASCII decimal değerleri
    }
}