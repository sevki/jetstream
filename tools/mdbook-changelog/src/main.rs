use mdbook_core::book::{Book, BookItem};
use mdbook_preprocessor::{Preprocessor, PreprocessorContext};
use regex::Regex;
use std::io;
use std::process;

fn main() {
    let preprocessor = ChangelogPreprocessor;

    if std::env::args().nth(1).as_deref() == Some("supports") {
        // Signal that we support all renderers
        process::exit(0);
    }

    let (ctx, book): (PreprocessorContext, Book) =
        serde_json::from_reader(io::stdin()).expect("Failed to parse input");

    let processed = preprocessor
        .run(&ctx, book)
        .expect("Failed to process book");

    serde_json::to_writer(io::stdout(), &processed)
        .expect("Failed to write output");
}

struct ChangelogPreprocessor;

impl Preprocessor for ChangelogPreprocessor {
    fn name(&self) -> &str {
        "changelog"
    }

    fn run(
        &self,
        _ctx: &PreprocessorContext,
        mut book: Book,
    ) -> anyhow::Result<Book> {
        // Regex to match ## [version](url) (date) and convert to ## version (date)
        // Multiline mode so ^ matches start of each line
        let re = Regex::new(r"(?m)^(##\s+)\[([^\]]+)\]\([^)]+\)(\s+\([^)]+\))")
            .unwrap();

        book.for_each_mut(|item| {
            if let BookItem::Chapter(chapter) = item {
                // Only process changelog chapters
                if chapter.name.to_lowercase().contains("changelog") {
                    chapter.content = re
                        .replace_all(&chapter.content, "${1}${2}${3}")
                        .to_string();
                }
            }
        });

        Ok(book)
    }
}
