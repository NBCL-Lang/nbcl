use nbcl::NbclEngine;
use std::env;
use std::fs;

fn main() {
    let path = env::args().nth(1).unwrap_or_else(|| {
        eprintln!("usage: nbcl <file.nbl>");
        std::process::exit(1);
    });

    let source = fs::read_to_string(&path).unwrap_or_else(|e| {
        eprintln!("could not read {path}: {e}");
        std::process::exit(1);
    });

    let engine = NbclEngine::new();

    match engine.parse_str(&source) {
        Ok(ast) => {
            println!("AST: {:#?}", ast);

            match engine.evaluate(ast) {
                Ok(evaled) => println!("{:#?}", evaled),
                Err(e) => println!("{}", e),
            }
        }
        Err(e) => eprintln!("{}", e),
    }
}
