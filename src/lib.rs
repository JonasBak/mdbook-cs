use mdbook::book::{Book, BookItem};
use mdbook::errors::Error;
use mdbook::preprocess::{Preprocessor, PreprocessorContext};
use regex::{Captures, Regex};
use std::fs::{self, File};
use std::io::prelude::*;
use std::path::Path;
use std::process::Command;

// TODO
// Set width and height
// Include title
// Maybe refer to images?
// Include link to source tex file?
// Create docker image that installs texlive, texlive-standalone, texlive-pgfplots

pub struct Preproc;

impl Preproc {
    pub fn new() -> Preproc {
        Preproc
    }
}

impl Preprocessor for Preproc {
    fn name(&self) -> &str {
        "mdbook-cs"
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
                let re = Regex::new(r"```cs-latex, *(\S+)\n([^`]*?)\n```").unwrap();

                ch.content = re
                    .replace_all(&ch.content, |caps: &Captures| {
                        let figure_name = &caps[1];
                        let content = &caps[2];

                        let mut file_path = build_path.join(figure_name);
                        file_path.set_extension("tex");

                        let mut f = File::create(file_path).unwrap();
                        f.write_all(
                            br"
                                \documentclass[tikz,border=10pt]{standalone}
                                \usepackage[utf8]{inputenc}
                                \usepackage{verbatim}
                                \usepackage{fancybox}
                                \usepackage{pgfplots}
                                \usepackage{tikz}
                                \usetikzlibrary{positioning,arrows,shapes,intersections,trees}
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
