use std::{io::Read, path::PathBuf};
use std::fs;
use std::process::Command;
use std::process::Stdio;
use cargo_metadata::*;
use cargo_metadata::diagnostic::DiagnosticCode;

//TODO: NFT your error book in one click...
fn main() {
    let args = std::env::args_os().skip(1);
//    let target_dir = PathBuf::from("/Users/gilescope/projects/cargo-errorbook/compile-errors/target/");
    let target_dir = PathBuf::from("target/");

    let mut cmd = Command::new("cargo");
    //cmd.current_dir("/Users/gilescope/projects/cargo-errorbook/compile-errors");
    cmd.arg("check");
    cmd.args(args);
    cmd.stdout(Stdio::piped());
    cmd.arg("--message-format");
    cmd.arg("json");
    let out = cmd.output().unwrap();
    let mut errors = vec![];
    let reader = std::io::BufReader::new(out.stdout.take(10_000_000));
    for message in cargo_metadata::Message::parse_stream(reader) {
        match message.unwrap() {
            Message::CompilerMessage(msg) => {
                //println!("{:#?}", msg);
                let rendered = msg.message.rendered.unwrap();
                let first = rendered.lines().next().unwrap();
                errors.push(Error{ name : first.to_owned(), rendered: rendered.clone(), code: msg.message.code });
                //println!("{:?}", first);
            },
            Message::CompilerArtifact(_artifact) => {
            //    println!("{:?}", artifact);
            },
            Message::BuildScriptExecuted(_script) => {
            //    println!("{:?}", script);
            },
            Message::BuildFinished(_finished) => {
              //  println!("{:?}", finished);
            },
            _ => () // Unknown message
        }
    }
    //let s = String::from_utf8_lossy(out.stderr.as_slice());

    write_book(errors, target_dir);
}

struct Error {
    name: String,
    rendered: String,
    code: Option<DiagnosticCode>
}

fn write_book(errors: Vec<Error>, target_dir: PathBuf) {
    let data = r#"
    [book]
    title = "Yet Another ErrorBook"
    description = "A shrine to the hard work of Esteban and friends."
    authors = ["errorbook"]
    language = "en"
    
    [rust]
    edition = "2018"
    
    [output.html]
    mathjax-support = false
    
    
    [output.html.search]
    limit-results = 20
    use-boolean-and = true
    boost-title = 2
    boost-hierarchy = 2
    boost-paragraph = 1
    expand = true
    heading-split-level = 2
    "#;
    fs::create_dir_all(target_dir.join("errorbook/src")).unwrap();
    fs::write(target_dir.join("errorbook/Cargo.toml"), data).expect("Unable to write file");

    let mut summary = format!(r##"
    # Summary

    "##);
    for (i, error) in errors.iter().enumerate() {
      summary.push_str(&format!("\n   - [{}]({}.md)", error.name, i));
      let mut error_page = format!(r##"# {}
```rust,noplaypen
{}
```

"##, error.name, error.rendered);
      if let Some(code) = &error.code {
          //https://doc.rust-lang.org/error-index.html#{}
          error_page.push_str(&format!("[Explain {} to me](https://duckduckgo.com/?q=rust+{}).", code.code, code.code));
      }
      let error_pg_filename = format!("errorbook/src/{}.md", i);
      fs::write(target_dir.join(error_pg_filename), error_page).expect("Unable to write file");
    }

    fs::write(target_dir.join("errorbook/src/Summary.md"), summary).expect("Unable to write file");

    let mut compile = Command::new("mdbook");
    compile.arg("build").current_dir(target_dir.join("errorbook"));
    compile.status().unwrap();
}