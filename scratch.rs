use rawk::parser::parse;
fn main() {
    let p = parse(r#"$1 ~ /^[0-9]+$/ { print "number:", $1 }"#).unwrap();
    println!("{:#?}", p);
}
