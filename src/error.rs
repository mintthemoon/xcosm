use cosmwasm_std::StdError;

use crate::{AuthError, CoinError, FundError, MathError, ValidateError};

/// Type alias for `std::result::Result` with contract defaults.
pub type XcosmResult<T=(), E=XcosmError> = std::result::Result<T, E>;

/// Contract error.
#[derive(thiserror::Error, Debug, miette::Diagnostic)]
pub enum XcosmError {
  /// Auth error.
  #[error(transparent)]
  Auth(#[from] AuthError),

  /// Coin error.
  #[error(transparent)]
  Coin(#[from] CoinError),

  /// Fund error.
  #[error(transparent)]
  Fund(#[from] FundError),

  /// Math error.
  #[error(transparent)]
  Math(#[from] MathError),

  /// Validate error.
  #[error(transparent)]
  Validate(#[from] ValidateError),

  /// CosmWasm standard error.
  #[error(transparent)]
  Std(#[from] StdError),

  /// Action disabled error.
  #[error("This action is disabled")]
  Disabled {},

  /// Input parsing error.
  #[error("Unable to parse input value")]
  Parse {},
}

impl Into<StdError> for XcosmError {
  /// Convert contract error into CosmWasm standard error.
  fn into(self) -> StdError {
    match self {
      XcosmError::Std(err) => err,
      _ => StdError::generic_err(self.to_string()),
    }
  }
}

/// Trait for conversions between result types.
pub trait IntoResult<T, E> {
  /// Convert result to target type.
  fn into_result(self) -> Result<T, E>;
}

impl<T, E, F: Into<E>> IntoResult<T, E> for Result<T, F> {
  fn into_result(self) -> Result<T, E> {
    self.map_err(Into::into)
  }
}

pub trait FromResult<T, E> {
  fn from_result(res: Result<T, E>) -> Self;
}

impl<T, E: Into<F>, F> FromResult<T, E> for Result<T, F> {
  fn from_result(res: Result<T, E>) -> Self {
    res.map_err(Into::into)
  }
}
