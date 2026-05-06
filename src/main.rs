use std::env;
use std::fs;
use nbcl::NbclEngine;

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
            println!("{:#?}", ast);
            let evaled = engine.evaluate(ast);
            println!("{:#?}", evaled);

        },
        Err(e)  => eprintln!("{e}"),
    }
}