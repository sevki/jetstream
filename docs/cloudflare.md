# JetStream Cloudflare

Let's say you've defined a service like so

```rs
{{#include ../components/jetstream_radar/src/lib.rs}}
```
The glue code for running it on Cloudflare Workers is
```rs
{{#include ../components/jetstream_radar/src/server.rs}}
```
The code for connecting to it is as follows:
```rs
{{#include ../components/jetstream_radar/src/bin/client.rs}}
```
