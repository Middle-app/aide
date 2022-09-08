//! Thread-local context for common settings for documentation generation.

use std::cell::RefCell;

use schemars::gen::{SchemaGenerator, SchemaSettings};

use crate::error::Error;

thread_local! {
    static GEN_CTX: RefCell<GenContext> = RefCell::new(GenContext::new());
}

/// Access the current thread-local context for
/// API documentation generation.
pub fn in_context<R, F>(cb: F) -> R
where
    F: FnOnce(&mut GenContext) -> R,
{
    GEN_CTX.with(|ctx| cb(&mut *ctx.borrow_mut()))
}

/// Register an error handler in the current thread-local context.
///
/// Only one handler is allowed at a time, this
/// function will overwrite the existing one.
///
/// Due to the design of the library in some cases
/// errors can be false positives that cannot be
/// avoided.
///
/// It is advised **not to panic** in this handler
/// unless you are interested in the backtrace for
/// a specific error.
pub fn on_error(handler: impl Fn(Error) + 'static) {
    in_context(|ctx| ctx.error_handler = Some(Box::new(handler)));
}

/// Collect common schemas in the thread-local context,
/// then store them under `#/components/schemas` the next
/// time generated content is merged into [`OpenApi`].
///
/// This will automatically clear the schemas stored
/// in the context.
///
/// [`OpenApi`]: crate::openapi::OpenApi
pub fn extract_schemas(extract: bool) {
    in_context(|ctx| {
        if extract {
            ctx.schema = SchemaGenerator::new(SchemaSettings::draft07().with(|s| {
                s.inline_subschemas = false;
                s.definitions_path = "#/components/schemas/".into();
            }));
            ctx.extract_schemas = true;
        } else {
            ctx.schema = SchemaGenerator::new(SchemaSettings::draft07().with(|s| {
                s.inline_subschemas = true;
            }));
            ctx.extract_schemas = false;
        }
    });
}

    });
}

/// Reset the state of the thread-local context.
///
/// Currently clears:
///
/// - extracted schemas
///
/// This function is not required in most cases.
pub fn reset_context() {
    in_context(|ctx| *ctx = GenContext::new());
}

/// A context for API document generation
/// that provides settings and a [`SchemaGenerator`].
pub struct GenContext {
    /// Schema generator that should be used
    /// for generating JSON schemas.
    pub schema: SchemaGenerator,

    /// Extract schemas.
    pub(crate) extract_schemas: bool,

    /// The following filter is used internally
    /// to reduce the amount of false positives
    /// when possible.
    pub(crate) show_error: fn(&Error) -> bool,
    error_handler: Option<Box<dyn Fn(Error)>>,
}

impl GenContext {
    fn new() -> Self {
        Self {
            schema: SchemaGenerator::new(
                SchemaSettings::draft07().with(|s| s.inline_subschemas = true),
            ),
            extract_schemas: false,
            show_error: default_error_filter,
            error_handler: None,
        }
    }

    pub(crate) fn reset_error_filter(&mut self) {
        self.show_error = default_error_filter;
    }

    /// Add an error in the current context.
    #[tracing::instrument(skip_all)]
    pub fn error(&mut self, error: Error) {
        if let Some(handler) = &self.error_handler {
            if !(self.show_error)(&error) {
                return;
            }

            handler(error);
        }
    }
}

fn default_error_filter(_: &Error) -> bool {
    true
}
