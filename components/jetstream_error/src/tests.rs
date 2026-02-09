#[cfg(feature = "test-paths")]
#[tracing::instrument]
fn validate_request(user_id: u64, email: &str) -> super::Error {
    super::Error::with_code(
        "Server-side validation failed",
        "server::validation::E001",
    )
}

#[cfg(feature = "test-paths")]
#[tracing::instrument]
fn handle_request(endpoint: &str) -> super::Error {
    validate_request(42, "user@example.com")
}

#[cfg(feature = "test-paths")]
#[tracing::instrument]
fn make_error() -> super::Error {
    handle_request("/api/v1/submit")
}

/// r[verify jetstream.error.v2.error-layer]
/// r[verify jetstream.error.v2.span-trace.capture]
/// r[verify jetstream.error.v2.reporting.miette]
/// r[verify jetstream.error.v2.reporting.related]
/// r[verify jetstream.error.v2.reporting.span-trace-section]
/// r[verify jetstream.error.v2.source-info.miette-integration]
/// r[verify jetstream.error.v2.source-info.client-render]
/// r[impl jetstream.error.v2.error-layer]
/// r[impl jetstream.test.error-propagation.span-trace-capture]
/// r[verify jetstream.test.error-propagation.span-trace-capture]
#[test]
#[cfg(all(feature = "test-paths", feature = "miette"))]
fn test_error() {
    use insta::assert_snapshot;
    use term_transcript::{
        svg::{NamedPalette, Template, TemplateOptions},
        Interaction, Transcript,
    };
    use tracing_error::ErrorLayer;
    use tracing_subscriber::prelude::*;

    // Set up a tracing subscriber with ErrorLayer so SpanTrace captures spans
    let _ = tracing_subscriber::registry()
        .with(ErrorLayer::default())
        .try_init();

    // Create the error inside nested spans so the backtrace is populated
    let err = make_error();

    let h = miette::GraphicalReportHandler::new_themed(
        miette::GraphicalTheme::unicode(),
    );
    let mut output = String::new();
    h.render_report(&mut output, &err).unwrap();
    let mut transcript = Transcript::new();
    assert_snapshot!(output,@r#"
    [31mserver::validation::E001[0m

      [31m×[0m [server::validation::E001] Server-side validation failed

    Error: 
      [31m×[0m validate_request
       ╭─[[36;1;4mcomponents/jetstream_error/src/tests.rs:3:4[0m]
     [2m2[0m │ #[tracing::instrument]
     [2m3[0m │ fn validate_request(user_id: u64, email: &str) -> super::Error {
       · [35;1m   ────────┬───────[0m
       ·            [35;1m╰── [35;1mvalidate_request(user_id: 42, email: "user@example.com")[0m[0m
     [2m4[0m │     super::Error::with_code(
       ╰────

    Error: 
      [31m×[0m handle_request
        ╭─[[36;1;4mcomponents/jetstream_error/src/tests.rs:12:4[0m]
     [2m11[0m │ #[tracing::instrument]
     [2m12[0m │ fn handle_request(endpoint: &str) -> super::Error {
        · [35;1m   ───────┬──────[0m
        ·           [35;1m╰── [35;1mhandle_request(endpoint: "/api/v1/submit")[0m[0m
     [2m13[0m │     validate_request(42, "user@example.com")
        ╰────

    Error: 
      [31m×[0m make_error
        ╭─[[36;1;4mcomponents/jetstream_error/src/tests.rs:18:4[0m]
     [2m17[0m │ #[tracing::instrument]
     [2m18[0m │ fn make_error() -> super::Error {
        · [35;1m   ─────┬────[0m
        ·         [35;1m╰── [35;1mmake_error()[0m[0m
     [2m19[0m │     handle_request("/api/v1/submit")
        ╰────
    "#);
    let interaction = Interaction::new("# do some rpc", output);
    transcript.add_existing_interaction(interaction);

    let template_options = TemplateOptions {
        palette: NamedPalette::Dracula.into(),

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
