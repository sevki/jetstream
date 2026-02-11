# Migrating to Dodeca

This document outlines the process of migrating JetStream documentation from mdbook to dodeca.

## Why Dodeca?

Dodeca is a modern, incremental static site generator that offers:
- Fast, incremental builds
- Production-like development environment
- Built-in image optimization, font subsetting, and HTML minification
- Custom Jinja-like template engine
- Plugin architecture

See [dodeca.bearcove.eu](https://dodeca.bearcove.eu/) for more information.

## Current Status

✅ **Completed:**
- Created dodeca-compatible directory structure in `site/`
- Migrated core documentation pages (QUIC, Iroh, HTTP)
- Added TOML frontmatter to markdown files
- Created basic HTML templates
- Copied CSS styles

🚧 **In Progress:**
- Installing dodeca build tools
- Setting up CI/CD pipeline

📋 **TODO:**
- Migrate remaining content (changelog, crates documentation)
- Create additional templates for different page types
- Set up search functionality
- Configure proper asset pipeline

## Directory Structure

```
site/
├── content/          # Markdown content with TOML frontmatter
│   ├── _index.md    # Homepage
│   ├── quic.md      # QUIC documentation
│   ├── iroh.md      # Iroh documentation
│   └── http.md      # HTTP documentation
├── templates/        # HTML templates
│   ├── base.html    # Base template
│   └── page.html    # Page template
├── static/           # Static assets
│   └── styles.css   # Stylesheet
├── config.toml       # Site configuration
└── README.md         # Documentation
```

## Installation

### Option 1: From Source (Recommended for now)

```bash
# Clone dodeca
git clone https://github.com/bearcove/dodeca.git
cd dodeca

# Add wasm target
rustup target add wasm32-unknown-unknown

# Build
cargo xtask build

# Install
cargo install --path crates/dodeca
```

### Option 2: From Releases (When Available)

```bash
# Install via script (macOS/Linux)
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/bearcove/dodeca/releases/latest/download/dodeca-installer.sh | sh
```

## Building the Site

### Local Development

```bash
cd site
ddc serve
```

Visit `http://localhost:8080` to view the site with live reload.

### Production Build

```bash
cd site
ddc build
```

Output will be in `site/public/`.

## Migration Checklist

- [ ] Install dodeca CLI
- [ ] Test local builds
- [ ] Migrate changelog content
- [ ] Migrate crates documentation
- [ ] Create section templates
- [ ] Set up syntax highlighting
- [ ] Configure image optimization
- [ ] Update CI/CD workflow
- [ ] Test deployment to GitHub Pages
- [ ] Update README with new instructions
- [ ] Archive old mdbook setup

## Keeping Both Systems

During the transition, both mdbook and dodeca setups can coexist:

- **mdbook**: `docs/` directory, built with `mdbook build`
- **dodeca**: `site/` directory, built with `ddc build`

This allows for gradual migration and comparison.

## CI/CD Integration

A GitHub Actions workflow has been created at `.github/workflows/dodeca.yml` that:
1. Installs Rust and dodeca
2. Builds the site
3. Generates API documentation
4. Deploys to GitHub Pages

## Resources

- [Dodeca Documentation](https://dodeca.bearcove.eu/)
- [Dodeca GitHub](https://github.com/bearcove/dodeca)
- [JetStream Site Structure](./site/README.md)

## Questions or Issues?

If you encounter issues during migration, please open a GitHub issue or reach out to the team.
