use rawk::parser::parse_program;

fn main() {
    let code = "BEGIN { for(i=1; i<=3; i++) print \"Ciao\", i; }";
    match parse_program(code) {
        Ok(prog) => println!("{:#?}", prog),
        Err(e) => println!("Parse error: {}", e),
    }
}
