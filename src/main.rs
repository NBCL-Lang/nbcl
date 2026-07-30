use std::path::PathBuf;
use nbcl::NbclEngine;
use std::env;
use std::fs;

fn main() {
    let path = env::args().nth(1).unwrap_or_else(|| {
        eprintln!("usage: nbcl <file.nbl>");
        std::process::exit(1);
    });

    let path_buf = PathBuf::from(path.clone());
    let source = fs::read_to_string(&path_buf).unwrap_or_else(|e| {
        eprintln!("could not read {path}: {e}");
        std::process::exit(1);
    });

    let show_ast = env::args().nth(2).unwrap_or(String::new());

    let mut engine = NbclEngine::new();
    engine.set_root_file(path_buf);

    match engine.parse_str(&source) {
        Ok(ast) => {
            if show_ast == "--show-ast" {
                println!("AST: {:#?}", ast);
            }

            match engine.evaluate_ast(ast) {
                Ok(evaled) => println!("{:#?}", evaled),
                Err(e) => println!("{}", e),
            }
        }
        Err(e) => eprintln!("{}", e),
    }
}
