//! Errors that can occur when determining an optimum label position

use thiserror::Error;

/// Possible Polylabel errors
#[derive(Error, Debug, PartialEq)]
#[error("{0}")]
pub enum PolylabelError {
    #[error("Couldn't calculate a centroid for the input Polygon")]
    CentroidCalculation,
    #[error("Couldn't calculate a bounding box for the input Polygon")]
    RectCalculation,
    #[error("The priority queue is unexpectedly empty. This is a bug!")]
    EmptyQueue,
}
