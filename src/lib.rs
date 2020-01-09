use rustc_errors::{Diagnostic, Handler};
use rustc_session::parse::ParseSess;
use rustc_span::source_map::{FileName, FilePathMapping, SourceMap};
use std::rc::Rc;
use syntax::errors::emitter::Emitter;

/// Emitter which discards every error.
struct SilentEmitter;

impl Emitter for SilentEmitter {
    fn emit_diagnostic(&mut self, _db: &Diagnostic) {}
    fn source_map(&self) -> Option<&Rc<SourceMap>> {
        None
    }
}

pub fn parse(text: String) {
    use std::sync::{Arc, Mutex};
    let non_diag_panic = Arc::new(Mutex::new(None));

    {
        let non_diag_panic = non_diag_panic.clone();

        std::panic::set_hook(Box::new(move |panic_info| {
            if let Some(location) = panic_info.location() {
                if location.file().ends_with("/diagnostic_builder.rs") {
                    return;
                }
            }
            if let Ok(mut inner) = non_diag_panic.lock() {
                *inner = Some(format!("{:?}", panic_info));
            }
        }));
    }

    let _ = std::panic::catch_unwind(|| {
        syntax::with_globals(rustc_span::edition::Edition::Edition2018, || {
            let source_map = Rc::new(SourceMap::new(FilePathMapping::empty()));
            let tty_handler = Handler::with_emitter(true, None, Box::new(SilentEmitter));
            let sess = ParseSess::with_span_handler(tty_handler, source_map);
            let parser = rustc_parse::maybe_new_parser_from_source_str(
                &sess,
                FileName::Custom("stdin".to_owned()),
                text,
            );
            if let Ok(mut p) = parser {
                let _ = p.parse_crate_mod();
            }
        });
    });

    if let Ok(p) = non_diag_panic.clone().lock() {
        if let Some(p) = p.as_ref() {
            panic!("{}", p);
        }
    }

    let _ = std::panic::take_hook();
}
