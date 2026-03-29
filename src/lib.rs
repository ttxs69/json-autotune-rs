//! # JSON-AutoTune
//!
//! JSON parser with SIMD optimization, auto-tuned by AI.
//!
//! ## Example
//!
//! ```rust
//! use json_autotune::parse;
//!
//! let value = parse(r#"{"name": "Alice", "age": 30}"#).unwrap();
//! assert_eq!(value["name"].as_str(), Some("Alice"));
//! ```

use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

pub mod parser;
mod value;
mod error;
mod simd;
mod number;

pub use value::Value;
pub use error::Error;
pub use parser::parse;