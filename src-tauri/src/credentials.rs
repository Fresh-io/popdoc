// OAuth credentials baked in at compile time from the `.env` file at the
// repo root. See `build.rs` for the wiring. No real values live in this
// file — the actual strings come from environment variables resolved by
// `env!()` at compile time. Missing values become empty strings and the
// app refuses to authenticate with a clear error message.

pub const EMBEDDED_CLIENT_ID: &str = env!("GDL_CLIENT_ID");
pub const EMBEDDED_CLIENT_SECRET: &str = env!("GDL_CLIENT_SECRET");
