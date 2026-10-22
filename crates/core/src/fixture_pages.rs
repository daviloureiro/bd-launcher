//! The pages of the apps the tests work with: the copy the tests were written against.
//! Generated from the suite the launcher used to ship; kept here so the tests keep their
//! expectations while the catalog itself is remote.

/// One page of an app: its headline, its story, its features and its pictures.
pub struct Page {
    pub id: &'static str,
    pub headline: &'static str,
    pub about: &'static str,
    pub features: &'static [(&'static str, &'static str)],
    pub shots: &'static [(&'static str, &'static str)],
}

/// The pages, in the order the suite had them.
pub const PAGES: &[Page] = &[
    Page {
        id: "photocraft",
        headline: "The image editor you already know how to use.",
        about: "Layers, masks, adjustment layers, layer styles, type, vectors and brushes in a native app written in Rust. Open source, offline, and yours.\n\nPhotoCraft keeps the menus, shortcuts and panels your hands already know, so there is nothing to relearn.",
        features: &[
            ("Native GPU canvas", "No Electron and no web view: a fast native compositor."),
            ("Real layered files", "Opens and saves layered PSD and PSB documents."),
            ("Non-destructive editing", "Adjustment layers, live-preview filters, smart objects and layer styles."),
            ("On-device selection", "Subject and object selection and mask refinement run locally."),
            ("Serious color", "8-, 16- and 32-bit RGB, CMYK, Lab and Grayscale, with ICC color management and soft proofing."),
        ],
        shots: &[
            ("demo", "Curves and type layers"),
            ("adjustments", "Levels with a live histogram"),
            ("layer-styles", "Layer styles on live type"),
            ("masks", "Selections and masked adjustments"),
        ],
    },
    Page {
        id: "vectorcraft",
        headline: "Vector illustration, reimagined in pure Rust.",
        about: "A native vector illustration app with the pen, panels and shortcuts illustrators expect. Open source, and it also runs in a browser.",
        features: &[
            ("Genuinely fast", "Drawings with thousands of shapes stay responsive on a retina display."),
            ("Live, editable effects", "Blends, envelope distort, gradient mesh, repeats, glows and drop shadows all stay editable."),
            ("Image tracing", "Turn raster images into clean vector paths, with presets for the common cases."),
            ("Open formats", "Native JSON documents, SVG and PDF import and export, and PNG, JPEG and WebP output."),
        ],
        shots: &[
            ("neon", "The Neon Drive poster"),
            ("ribbons", "Live blends"),
            ("sheet", "A feature sheet and the Layers panel"),
            ("bezier", "Pathfinder and exact booleans"),
        ],
    },
    Page {
        id: "filmcraft",
        headline: "Professional video editing, rebuilt from scratch.",
        about: "A native video editor written in pure Rust, with source and program monitors, a real timeline and every trim mode.",
        features: &[
            ("A timeline you already know", "Source and program monitors, three-point editing, every trim mode and J/K/L trimming."),
            ("Frame-exact time", "An integer timebase keeps 23.976 and 29.97 drop-frame footage exact, with no drift."),
            ("Its own codecs", "No FFmpeg inside: native support for H.264, HEVC, ProRes, VP9, AAC and Opus."),
            ("Color grading", "Curves, color wheels, HSL secondaries, LUTs and HDR and log workflows, with scopes to check the result."),
            ("Broadcast-ready audio", "EBU R128 loudness meters, a mixer with automation and native audio effects."),
        ],
        shots: &[
            ("color", "The Color workspace, with scopes"),
            ("assembly", "The Assembly workspace"),
            ("keyframes", "Keyframes in Effect Controls"),
        ],
    },
    Page {
        id: "lightcraft",
        headline: "Your photos. Your pixels. Your machine.",
        about: "A fast photo library and non-destructive raw developer, written from scratch in pure Rust. No account, no cloud, no telemetry and no subscription.",
        features: &[
            ("Non-destructive by design", "A wide-gamut, 32-bit float pipeline that never touches your originals."),
            ("Every control you reach for", "Light, color, effects, tone curve, a color mixer, color grading wheels and built-in presets."),
            ("Precise masking", "Brush, gradient and range masks, plus sky, subject and background masks."),
            ("Native raw decoding", "Pure-Rust decoders for the main camera raw formats: DNG, CR2, ARW, NEF, RAF, RW2, PEF and ORF."),
            ("A library that keeps up", "Albums, ratings, flags and search, with grids that stay smooth on large catalogs."),
            ("Instant feedback", "The GPU pipeline updates the preview as you move a slider, even on large raw files."),
        ],
        shots: &[
            ("hero", "The Edit view"),
            ("curve", "The tone curve"),
            ("grading", "Color grading wheels"),
            ("masking", "A radial mask over a sky mask"),
        ],
    },
    Page {
        id: "pdfcraft",
        headline: "The open-source PDF workbench.",
        about: "Read, organize, combine, split and secure PDFs in a fast, native app written in Rust from the ground up. No account, no telemetry and no cloud.",
        features: &[
            ("Faithful rendering", "World scripts, vertical Japanese, color emoji and transparency."),
            ("Search as you type", "Instant find, and text selection that follows the document's real reading order."),
            ("Organize like cards", "Rotate, delete, insert and reorder pages, with deep undo behind every change."),
            ("Combine, extract, split", "Assemble and break apart documents while keeping links, form fields, layers and bookmarks intact."),
            ("Fearless saves", "Incremental, atomic saves with autosave and crash recovery."),
            ("Built-in security", "Opens files from RC4 to AES-256 encryption and honors the author's permissions."),
        ],
        shots: &[],
    },
    Page {
        id: "effectcraft",
        headline: "Motion graphics and visual effects, in pure Rust.",
        about: "A native compositor with compositions, layers, keyframes, effects, expressions, 3D cameras and lights, and a render queue. Young, moving fast, and already usable.",
        features: &[
            ("Real 3D", "3D layers with cameras, depth of field, and lights that cast soft ray-traced shadows."),
            ("Expressions", "JavaScript expressions with wiggle, loops and layer references, edited inline."),
            ("Export anywhere", "H.264 and ProRes with alpha, PNG, TIFF and EXR sequences, GIF, and Lottie import and export."),
        ],
        shots: &[
            ("hero", "An animated composition"),
            ("graph", "The Graph Editor"),
            ("effects", "Particles and Effect Controls"),
            ("3d", "3D layers with cameras and lights"),
        ],
    },
    Page {
        id: "designcraft",
        headline: "Page layout and publishing, rebuilt in pure Rust.",
        about: "A native layout app with spreads, parent pages, threaded stories, styles, swatches, text wrap and a paragraph composer. No subscription, no license server and no telemetry.",
        features: &[
            ("The layout tools you know", "Spreads and parent pages, frames and threaded stories, with paragraph and character styles."),
            ("Print-ready PDF", "Real selectable text with embedded fonts, CMYK and spot colors, bleed, crop marks and PDF/A output."),
            ("Fast", "Multithreaded rendering, instant undo and cached text composition."),
            ("Open formats", "A documented native format, PNG export and layout interchange import and export."),
            ("Built for automation", "Every menu item, tool, panel and dialog can be driven over a JSON control channel and an MCP server."),
        ],
        shots: &[("spread", "A magazine spread with a threaded story")],
    },
    Page {
        id: "wordcraft",
        headline: "The familiar word processor, native and offline.",
        about: "A word processor with the ribbon, shortcuts and behavior you already know, written in Rust. Open source, and built from public specifications.",
        features: &[
            ("Fast", "Paragraph layout is cached, so typing stays responsive even in long documents."),
            ("Private", "Spelling, grammar and everything else work offline."),
            ("Everywhere", "One Rust codebase for macOS, Windows, Linux, BSD and the web."),
        ],
        shots: &[
            ("hero", "The Home tab and the Navigation pane"),
            ("review", "Track Changes and comments"),
            ("design", "The Design tab"),
            ("layout", "The Layout tab"),
        ],
    },
    Page {
        id: "deckcraft",
        headline: "Presentations, rebuilt in Rust.",
        about: "A presentation app with slide masters, shapes, animations and a presenter view, written in Rust. In alpha, heading for beta.",
        features: &[
            ("Make decks", "Slide masters and layouts with placeholders that inherit formatting, themes, sections, notes and comments."),
            ("Write", "In-place text editing with full font and paragraph formatting, bullets and numbering, columns and autofit."),
            ("Add content", "Pictures with crop and corrections, styled tables, charts, audio and video."),
            ("Exchange files", "Opens and saves .pptx presentation files and exports PDF (slides, notes pages and handouts), PNG and JPEG."),
        ],
        shots: &[
            ("editor", "Editing a slide, with sections and notes"),
            ("shape", "The Shape Format tab"),
            ("sorter", "The slide sorter"),
            ("format", "The Format Shape pane"),
        ],
    },
    Page {
        id: "gridcraft",
        headline: "Spreadsheets you already know how to use.",
        about: "A spreadsheet with the ribbon, formula bar, sheet tabs and shortcuts you know, on a calculation engine written in Rust. No subscription, no license server and no telemetry.",
        features: &[
            ("Fast", "Copy-on-write workbooks, incremental recalculation and a grid that only lays out what is on screen."),
            ("Everywhere", "One Rust codebase for the desktop and the web."),
        ],
        shots: &[
            ("hero", "A quarterly sales dashboard"),
            ("budget", "A household budget"),
            ("pivot", "A pivot table"),
            ("chart", "Formatting a chart"),
        ],
    },
    Page {
        id: "cadcraft",
        headline: "Computer-aided drafting, in pure Rust.",
        about: "A drafting app with the command line, object snaps and workflow that drafters already know. Free, with no account and no subscription. In early, fast development.",
        features: &[
            ("Fast and native", "Pure Rust and egui, with no Electron and no web view."),
            ("Built for agents", "Every menu item, tool and prompt is a command, available over MCP, a JSON control channel or the CLI."),
        ],
        shots: &[("apartment", "An apartment floor plan"), ("bracket", "A mechanical bracket drawing"), ("layout", "A layout sheet")],
    },
    Page {
        id: "soundcraft",
        headline: "Recording, editing and mixing, in Rust.",
        about: "A digital audio workstation that follows the workflow audio engineers already know: an Edit window for arranging and editing, and a Mix window for balancing. Pre-alpha.",
        features: &[],
        shots: &[
            ("edit", "The Edit window with a nine-track session"),
            ("mix", "The Mix window"),
            ("automation", "Automation and memory locations"),
            ("score", "The Score Editor"),
        ],
    },
    Page {
        id: "figcraft",
        headline: "Interface design and prototyping, in pure Rust.",
        about: "Design screens, components and prototypes in a native app that opens instantly and works offline, with no account and no telemetry. Early alpha.",
        features: &[("Vectors", "Real vector networks, the pen and pencil, the shape builder, editable booleans and variable width strokes.")],
        shots: &[],
    },
];
