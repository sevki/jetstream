# JetStream Documentation Site

This directory contains the JetStream documentation using a dodeca-inspired structure.

## Structure

```
site/
├── content/          # Markdown content files
│   ├── _index.md    # Homepage
│   ├── quic.md      # QUIC transport documentation
│   ├── iroh.md      # Iroh transport documentation
│   └── http.md      # HTTP documentation
├── templates/        # HTML templates (to be added)
├── static/           # Static assets (images, CSS, JS)
└── config.toml       # Site configuration
```

## Building

To build the site with dodeca:

```bash
ddc build
```

To serve locally:

```bash
ddc serve
```

## Migration Notes

This structure follows dodeca's conventions:
- Markdown files in `content/` with TOML frontmatter (`+++`)
- Templates in `templates/` directory
- Static files in `static/` directory
- Site configuration in `config.toml`

## TODO

- [ ] Install dodeca (`cargo install dodeca` or use releases)
- [ ] Create HTML templates
- [ ] Migrate remaining documentation
- [ ] Update CI/CD pipeline
- [ ] Copy static assets (logos, styles)
