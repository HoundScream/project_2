# PROJECT_CONTEXT

## Project Purpose

`Geonetix` is currently a Rust + Axum application that renders HTML on the server.
There is no JSX/TSX layer in the repository yet, so the same separation principle is applied to Rust render functions:

- UI renderers in [`src/main.rs`](/home/studytool/dev/Geonetix/src/main.rs:1) should only compose markup.
- All visible copy, SEO text, metadata, CTA labels, form labels, and system-facing page descriptors should live in [`content/`](/home/studytool/dev/Geonetix/content/mod.rs:1).

This architecture is intentionally shaped so it can later be reused by:

- a React/Next.js frontend,
- a headless CMS,
- localization pipelines,
- multiple AI agents editing content in parallel.

## Current Structure

```text
Geonetix/
├── content/
│   ├── metadata.rs
│   ├── mod.rs
│   ├── schema.rs
│   ├── seo.rs
│   └── pages/
│       ├── landing.rs
│       ├── mod.rs
│       ├── not_found.rs
│       ├── pricing.rs
│       └── pro_package.rs
├── database/
├── deploy/
├── scripts/
├── src/
│   └── main.rs
├── Cargo.toml
└── PROJECT_CONTEXT.md
```

## Folder Responsibilities

- `content/`
  Single source of truth for textual content.
- `content/schema.rs`
  Reusable content contracts for pages and sections.
- `content/seo.rs`
  SEO-only data that should stay independent from UI wording.
- `content/metadata.rs`
  Stable page ids, routes, locale hints, CMS collection names.
- `content/pages/*`
  Page-scoped content grouped by sections such as `hero`, `features`, `faq`, `cta`, `footer`.
- `src/main.rs`
  Rendering logic, routes, handlers, data loading, server composition.
- `database/`
  SQL schema and DB bootstrap.
- `deploy/`
  Production deployment assets.
- `scripts/`
  Operational scripts only. Not a source of UI copy.

## Tone Of Voice

- Clear, confident, product-oriented.
- Practical rather than hype-driven.
- Explains value in plain language.
- Avoids vague marketing promises.
- Keeps analytical credibility for B2B/local SEO workflows.

## Writing Rules

- Keep one idea per sentence when possible.
- Prefer short, high-signal CTA labels.
- Avoid duplicate claims across sections.
- Reuse shared terminology consistently:
  `анализ`, `сценарий`, `мониторинг`, `карточка бизнеса`, `конкуренты`.
- Error and status messages should be calm, specific, and actionable.
- SEO descriptions should describe the page outcome, not generic branding.

## Naming Conventions

- Page content files: `content/pages/<page_name>.rs`
- Section constants: `HERO`, `FEATURES`, `FAQ`, `CTA`, `FOOTER`
- Titles for groups: `FEATURES_TITLE`, `FAQ_TITLE`, `FLOW_TITLE`
- SEO objects: `PAGE.seo`
- Metadata objects: `PAGE.metadata`
- Shared schema types use noun-based names:
  `HeroContent`, `FeatureItem`, `FaqItem`, `CtaContent`, `FooterContent`

## UX/UI Principles

- UI components/renderers must not own product copy.
- Section order should stay predictable for editors:
  `hero -> features -> faq -> cta -> footer`
- Actions should be obvious and low-friction.
- Explanatory text should reduce uncertainty before forms and paid steps.
- System pages should use the same content discipline as marketing pages.
- Data attributes like `data-page-id`, `data-page-route`, and `data-cms-collection` should stay available for future CMS bindings and agent-safe automation.

## AI Editing Rules

- Prefer editing `content/pages/*` for text changes.
- Do not change HTML structure in `src/main.rs` if only copy needs updating.
- Do not introduce new inline user-facing strings into render functions unless there is a documented technical reason.
- Extend `content/schema.rs` before adding page-specific ad hoc text fields.
- Keep page ids and route metadata stable unless a route truly changes.
- Avoid broad rewrites of `src/main.rs` when a targeted content edit is enough.
- If a new page is added, create its content file first, then wire UI to that file.

## How To Update Content Without Changing UI

1. Open the target file in `content/pages/`.
2. Update the relevant section constant only.
3. Keep the existing schema shape if the UI already depends on it.
4. Run `cargo check`.
5. Only touch `src/main.rs` if a genuinely new section or schema field is needed.

## Multi-Agent Workflow

- Agent 1 can edit `content/pages/landing.rs`.
- Agent 2 can edit `content/pages/pricing.rs`.
- Agent 3 can extend `content/schema.rs` if a new shared section is required.
- Agent 4 can wire new fields in `src/main.rs`.

This minimizes merge conflicts because content is page-local and schema changes are isolated.

## CMS Readiness

The current Rust constants are a transitional CMS-ready layer.
To connect a CMS later:

- keep schema types in `content/schema.rs`,
- swap constant-backed page modules for deserialized CMS payloads,
- preserve `PAGE.metadata.id` and section names as stable content keys,
- keep SEO and metadata separate from visible body copy.

## Files To Avoid Changing Without Need

- [`src/main.rs`](/home/studytool/dev/Geonetix/src/main.rs:1)
  Change only for rendering logic, routing, or schema wiring.
- [`database/init/001_init.sql`](/home/studytool/dev/Geonetix/database/init/001_init.sql:1)
  Do not touch for copy tasks.
- [`deploy/docker-compose.prod.yml`](/home/studytool/dev/Geonetix/deploy/docker-compose.prod.yml:1)
  Deployment-only.
- [`Dockerfile`](/home/studytool/dev/Geonetix/Dockerfile:1)
  Infra-only.
- [`Cargo.toml`](/home/studytool/dev/Geonetix/Cargo.toml:1)
  Only update if the architecture truly requires new dependencies.

## Scaling Guidance

- Add new page content as `content/pages/<page>.rs`.
- Move repeated text into shared content modules only when repetition is real.
- Introduce locale folders only after 2+ languages are needed.
- If a future frontend appears, keep the same section names and page ids.
- For large sites, split `content/pages/` further into route groups:
  `marketing/`, `auth/`, `cabinet/`, `system/`.
