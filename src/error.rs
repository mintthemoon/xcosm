use cosmwasm_std::StdError;

use crate::{AuthError, CoinError, FundError, MathError, ValidateError};

/// Type alias for `std::result::Result` with contract defaults.
pub type CosmixResult<T=(), E=CosmixError> = std::result::Result<T, E>;

/// Contract error.
#[derive(thiserror::Error, Debug, miette::Diagnostic)]
pub enum CosmixError {
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

impl<'a> Into<StdError> for CosmixError {
  /// Convert contract error into CosmWasm standard error.
  fn into(self) -> StdError {
    match self {
      CosmixError::Std(err) => err,
      _ => StdError::generic_err(self.to_string()),
    }
  }
}

/// Trait for conversions between result types.
pub trait IntoResult<T, E> {
  /// Convert result to target type.
  fn into_result(self) -> Result<T, E>;
}

impl<T, E, F: From<E>> IntoResult<T, F> for Result<T, E> {
  /// Convert contract result to CosmWasm standard result.
  fn into_result(self) -> Result<T, F> {
    self.map_err(F::from)
  }
}

pub trait FromResult<T, E> {
  fn from_result(res: Result<T, E>) -> Self;
}

impl<T, E, F: Into<E>> FromResult<T, F> for Result<T, E> {
  fn from_result(res: Result<T, F>) -> Self {
    res.map_err(Into::into)
  }
}
