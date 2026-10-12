pub mod catalog;
pub mod elevate;
pub mod markdown;
pub mod status;

use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum CatalogError {
    #[error("cannot read catalog folder '{path}': {detail}")]
    ReadDir { path: String, detail: String },
}
