use super::*;

#[test]
#[cfg(feature = "test-paths")]
fn test_error() {
    use insta::assert_snapshot;
    use term_transcript::{
        svg::{NamedPalette, Template, TemplateOptions},
        Interaction, Transcript,
    };
    let _source = "Cpp is the best";
    let label = LabeledSpan::at(0..3, "should be Rust");
    // Return an error with full diagnostic information
    let err = Error::new("Server-side validation failed")
        .with_code("server::validation::E001")
        .with_severity(Severity::Error)
        .with_help("Check your input parameters")
        .with_label(label);

    let h = miette::GraphicalReportHandler::new();
    let mut output = String::new();
    h.render_report(&mut output, &err).unwrap();
    let mut transcript = Transcript::new();
    assert_snapshot!(output,@r"
    ]8;;file:///root/test_dir/components/jetstream_error/src/tests.rs:14:15\[31mserver::validation::E001 [0m[36;1;4m(link)[0m]8;;\

      [31m×[0m Server-side validation failed
    [36m  help: [0mCheck your input parameters
    ");
    let interaction = Interaction::new("# do some rpc", output);
    transcript.add_existing_interaction(interaction);

    let template_options = TemplateOptions {
        palette: NamedPalette::PowerShell.into(),

        ..TemplateOptions::default()
    };
    let mut buffer = vec![];
    Template::pure_svg(template_options)
        .render(&transcript, &mut buffer)
        .expect("failed to render template");
    let svg_data =
        String::from_utf8(buffer).expect("Failed to convert buffer to string");

    #[cfg(feature = "update-svg")]
    {
        // Write the SVG to src/ so rustdoc can find it
        let svg_path = concat!(env!("CARGO_MANIFEST_DIR"), "/src/output.svg");
        std::fs::write(svg_path, &svg_data).expect("Failed to write SVG file");
        // Read it back and snapshot
        let svg_content =
            std::fs::read_to_string(svg_path).expect("Failed to read SVG file");
        assert_snapshot!(svg_content);
    }

    #[cfg(not(feature = "update-svg"))]
    {
        assert_snapshot!(svg_data);
    }
}
