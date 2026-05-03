use pest::Parser;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "awk.pest"]
pub struct AwkParser;

fn main() {
    let input = "print $2, \"is\"";
    match AwkParser::parse(Rule::statement, input) {
        Ok(pairs) => println!("Success!"),
        Err(e) => println!("Error: {}", e),
    }
}
