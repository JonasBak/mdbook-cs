use crate::nop_lib::Nop;
use clap::{App, Arg, ArgMatches, SubCommand};
use mdbook::book::Book;
use mdbook::errors::Error;
use mdbook::preprocess::{CmdPreprocessor, Preprocessor, PreprocessorContext};
use regex::{Captures, Regex};
use std::io;
use std::process;

pub fn make_app() -> App<'static, 'static> {
    App::new("nop-preprocessor")
        .about("A mdbook preprocessor which does precisely nothing")
        .subcommand(
            SubCommand::with_name("supports")
                .arg(Arg::with_name("renderer").required(true))
                .about("Check whether a renderer is supported by this preprocessor"),
        )
}

fn main() {
    let matches = make_app().get_matches();

    // Users will want to construct their own preprocessor here
    let preprocessor = Nop::new();

    if let Some(sub_args) = matches.subcommand_matches("supports") {
        handle_supports(&preprocessor, sub_args);
    } else if let Err(e) = handle_preprocessing(&preprocessor) {
        eprintln!("{}", e);
        process::exit(1);
    }
}

fn handle_preprocessing(pre: &dyn Preprocessor) -> Result<(), Error> {
    let (ctx, book) = CmdPreprocessor::parse_input(io::stdin())?;

    if ctx.mdbook_version != mdbook::MDBOOK_VERSION {
        // We should probably use the `semver` crate to check compatibility
        // here...
        eprintln!(
            "Warning: The {} plugin was built against version {} of mdbook, \
             but we're being called from version {}",
            pre.name(),
            mdbook::MDBOOK_VERSION,
            ctx.mdbook_version
        );
    }

    let processed_book = pre.run(&ctx, book)?;
    serde_json::to_writer(io::stdout(), &processed_book)?;

    Ok(())
}

fn handle_supports(pre: &dyn Preprocessor, sub_args: &ArgMatches) -> ! {
    let renderer = sub_args.value_of("renderer").expect("Required argument");
    let supported = pre.supports_renderer(&renderer);

    // Signal whether the renderer is supported by exiting with 1 or 0.
    if supported {
        process::exit(0);
    } else {
        process::exit(1);
    }
}

/// The actual implementation of the `Nop` preprocessor. This would usually go
/// in your main `lib.rs` file.
mod nop_lib {
    use super::*;
    use mdbook::book::{Book, BookItem, Chapter};
    use std::fs::{self, File};
    use std::io::prelude::*;
    use std::path::{Path, PathBuf};
    use std::process::{Command, Stdio};

    /// A no-op preprocessor.
    pub struct Nop;

    impl Nop {
        pub fn new() -> Nop {
            Nop
        }
    }

    impl Preprocessor for Nop {
        fn name(&self) -> &str {
            "nop-preprocessor"
        }

        fn run(&self, ctx: &PreprocessorContext, mut book: Book) -> Result<Book, Error> {
            // if let Some(nop_cfg) = ctx.config.get_preprocessor(self.name()) {
            // }

            let build_path = Path::new("./build");
            let generated_path = Path::new("./src/generated");

            if !build_path.is_dir() {
                fs::create_dir(build_path).unwrap();
            }
            if !generated_path.is_dir() {
                fs::create_dir(generated_path).unwrap();
            }

            book.for_each_mut(|item| {
                if let BookItem::Chapter(ref mut ch) = item {
                    let head_number = ch.number.as_ref();

                    // TODO width and height, maybe ref?
                    let re = Regex::new(r"```cs-latex, *(\S+)\n((.*\n*)*)\n```").unwrap();

                    ch.content = re
                        .replace_all(&ch.content, |caps: &Captures| {
                            let figure_name = &caps[1];
                            let content = &caps[2];

                            let mut file_path = build_path.join(figure_name);
                            file_path.set_extension("tex");

                            let mut f = File::create(file_path).unwrap();
                            f.write_all(
                                br"
                                \documentclass{standalone}
                                \usepackage[utf8]{inputenc}
                                \usepackage{tikz}
                                \usetikzlibrary{positioning}
                                \begin{document}
                                ",
                            )
                            .unwrap();
                            f.write_all(content.as_bytes()).unwrap();
                            f.write_all(
                                br"
                                \end{document}
                                ",
                            )
                            .unwrap();

                            let cmd = Command::new("/usr/bin/pdflatex")
                                .current_dir(&build_path)
                                .arg(&Path::new(figure_name).with_extension("tex"))
                                .output()
                                .expect("could not spawn pdflatex");

                            if !cmd.status.success() {
                                panic!("yeet");
                            }

                            let mut pdf_from = build_path.join(figure_name);
                            pdf_from.set_extension("pdf");
                            let mut pdf_to = generated_path.join(figure_name);
                            pdf_to.set_extension("pdf");

                            fs::copy(pdf_from, pdf_to).unwrap();

                            format!(
                                r#"<embed src="/generated/{}.pdf" width="600px" height="400px"/>"#,
                                figure_name
                            )
                        })
                        .into_owned();
                }
            });

            Ok(book)
        }

        fn supports_renderer(&self, renderer: &str) -> bool {
            renderer == "html"
        }
    }
}
