# JetStream Cloudflare (deprecated)

Cloudflare Workers support was removed, along with the `jetstream_radar`
example crate that this page walked through.

The page is kept because it was published under this path. Its three code
listings used to be `{{#include}}` directives pointing into
`components/jetstream_radar/`, which is no longer in the repository —
mdbook logged an error for each one and rendered the page with three
empty code blocks, so what stood here was not an example anyone could
follow.

For a current, working example of defining a service and connecting to it,
see [the QUIC and iroh examples](https://github.com/sevki/jetstream/tree/main/examples).
