use pest::Parser;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar_inline = r#"
expr = { term ~ (op ~ term)* }
term = { "a" | "b" | "c" }
op = { "+" | "" }
"#]
pub struct MyParser;

fn main() {
    let pairs = MyParser::parse(Rule::expr, "a b+c");
    println!("{:?}", pairs);
}
