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

mod parser;
mod value;
mod error;
mod simd;

pub use value::Value;
pub use error::Error;
pub use parser::parse;