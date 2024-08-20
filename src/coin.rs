use std::collections::btree_map::Entry;
use std::collections::BTreeMap;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
  to_json_binary, Addr, AnyMsg, BankMsg, Coin, Coins, CoinsError, CosmosMsg, Uint128,
};
use derive_deref::{Deref, DerefMut};
use serde::{ser::SerializeSeq, Deserialize, Deserializer, Serialize, Serializer};

use crate::TryMinusMut;

pub type CoinResult<T=()> = Result<T, CoinError>;

#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum CoinError {
  /// Coins do not meet the expected amount.
  #[error("Insufficient coins provided: expected {expected:?}")]
  Insufficient { expected: String },

  /// Expected no coins, but received some.
  #[error("Empty coins required")]
  NotEmpty {},

  /// Action requires exact coins.
  #[error("Exact coins required: {expected:?}")]
  NotExact { expected: String },

  /// Coins lists cannot have duplicate denoms.
  #[error("Duplicate denom in coins: {denom:?}")]
  DuplicateDenom { denom: String },

  /// Expected coins, but received none.
  #[error("Non-empty coins required")]
  Empty {},

  /// Input/output match error for sending coins.
  #[error("Input coins and output coins must have equal values")]
  IoMismatch {},

  /// Coin error which _should_ never occur.
  #[error("Unexpected coin error: {msg:?}")]
  Unexpected { msg: String },
}

impl From<CoinsError> for CoinError {
  /// Convert a [`CoinsError`] into a [`CoinError`].
  fn from(e: CoinsError) -> Self {
    match e {
      CoinsError::DuplicateDenom => CoinError::DuplicateDenom {
        denom: "".to_string(),
      },
    }
  }
}

/// Sorted and dupe-checked map of coins that serializes as a list.
#[derive(Debug, Clone, PartialEq, Eq, Deref, DerefMut)]
pub struct CoinSet(BTreeMap<String, Uint128>);

impl CoinSet {
  /// Create a new [`CoinSet`] from a map.
  pub fn new(coins: BTreeMap<String, Uint128>) -> Self {
    CoinSet(coins)
  }

  /// Insert the amount into the set.
  ///
  /// Requires the denom to not already be present.
  pub fn try_insert(&mut self, denom: &String, amount: Uint128) -> CoinResult<&mut Uint128> {
    match self.entry(denom.clone()) {
      Entry::Occupied(entry) => Err(CoinError::DuplicateDenom {
        denom: entry.key().to_string(),
      }),
      Entry::Vacant(entry) => Ok(entry.insert(amount)),
    }
  }

  /// Get an iterator that yields [`Coin`] instances from the [`CoinSet`]`.
  pub fn iter_coins(&self) -> impl Iterator<Item=Coin>+'_ {
    self.iter().map(|(denom, amount)| Coin::new(*amount, denom))
  }

  /// Get a [`Vec<Coin`] from the [`CoinSet`].
  pub fn into_vec(self) -> Vec<Coin> {
    self.iter_coins().collect()
  }

  /// Validate expected coins and return the actual amount of the matching denom if it is
  /// valid, or an error if the denom is not present or less than the expected amount.
  ///
  /// Require coins to contain the expected denom in at least the expected amount.
  pub fn expect_coin(&self, expected: &Coin) -> CoinResult<&Uint128> {
    self
      .get(&expected.denom)
      .filter(|&amount| amount >= &expected.amount)
      .ok_or_else(|| CoinError::Insufficient {
        expected: expected.denom.clone(),
      })
  }

  /// Require coins to contain only the expected denom at exactly the expected amount.
  pub fn expect_coin_exact(&self, expected: &Coin) -> CoinResult {
    if self.expect_coin(expected)? != &expected.amount {
      return Err(CoinError::NotExact {
        expected: expected.to_string(),
      });
    }
    Ok(())
  }

  /// Require coins to contain all the expected denoms in at least the expected amounts.
  pub fn expect_coins(&self, expected: &[Coin]) -> CoinResult<Vec<&Uint128>> {
    expected.iter().map(|c| self.expect_coin(c)).collect()
  }

  /// Require coins to contain only the expected denoms at exactly the expected amounts.
  pub fn expect_coins_exact(&self, expected: &[Coin]) -> CoinResult {
    let expected_coins = &CoinSet::try_from(expected)?;
    if self != expected_coins {
      return Err(CoinError::NotExact {
        expected: expected_coins.to_string(),
      });
    }
    Ok(())
  }

  /// Require coins to be empty.
  pub fn expect_none(&self) -> CoinResult {
    if !self.is_empty() {
      return Err(CoinError::NotEmpty {});
    }
    Ok(())
  }

  /// Require coins to not be empty.
  pub fn expect_some(&self) -> CoinResult<&Self> {
    if self.is_empty() {
      return Err(CoinError::Empty {});
    }
    Ok(self)
  }

  pub fn send(&self, to: &Addr) -> CoinResult<CosmosMsg> {
    match self.len() {
      0..1 => Ok(send_coin(
        self
          .iter_coins()
          .next()
          .ok_or_else(|| CoinError::Empty {})?,
        to,
      )),
      _ => Ok(send_coins(self.expect_some()?, to)),
    }
  }

  pub fn send_many(&self, from: &Addr, output: Vec<(&Addr, CoinSet)>) -> CoinResult<CosmosMsg> {
    send_coins_many(self, from, output)
  }
}

impl Default for CoinSet {
  /// Create a default (empty) [`CoinSet`].
  fn default() -> Self {
    CoinSet(BTreeMap::new())
  }
}

impl Serialize for CoinSet {
  fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
    let mut seq = serializer.serialize_seq(Some(self.len()))?;
    for coin in self.iter_coins() {
      seq.serialize_element(&coin)?;
    }
    seq.end()
  }
}

impl<'de> Deserialize<'de> for CoinSet {
  fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
    let raw: Vec<Coin> = Deserialize::deserialize(deserializer)?;
    CoinSet::try_from(raw).map_err(serde::de::Error::custom)
  }
}

impl std::fmt::Display for CoinSet {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(
      f,
      "{}",
      serde_json::to_string(self).map_err(|_| std::fmt::Error {})?
    )
  }
}

impl TryFrom<Coins> for CoinSet {
  type Error = CoinError;

  fn try_from(coins: Coins) -> CoinResult<Self> {
    coins.to_vec().try_into()
  }
}

impl TryFrom<Vec<Coin>> for CoinSet {
  type Error = CoinError;

  /// Create [`CoinSet`] from an unsorted `Vec<Coin>`.
  ///
  /// Requires the provided list to contain no duplicates.
  fn try_from(raw: Vec<Coin>) -> CoinResult<Self> {
    raw.as_slice().try_into()
  }
}

impl TryFrom<&[Coin]> for CoinSet {
  type Error = CoinError;

  /// Create a [`CoinSet`] from an unsorted `Coin` slice.
  ///
  /// Requires the provided list to contain no duplicates.
  fn try_from(raw: &[Coin]) -> CoinResult<Self> {
    let mut coins = Self::default();
    for coin in raw {
      coins.try_insert(&coin.denom, coin.amount)?;
    }
    Ok(coins)
  }
}

impl Into<Vec<Coin>> for CoinSet {
  /// Convert a [`CoinSet`] into a sorted `Vec<Coin>`.
  fn into(self) -> Vec<Coin> {
    self.iter_coins().collect()
  }
}

impl Into<Vec<Coin>> for &CoinSet {
  /// Convert a [`CoinSet`] into a sorted `Vec<Coin>`.
  fn into(self) -> Vec<Coin> {
    self.iter_coins().collect()
  }
}

/// Create bank send message for single coin.
pub fn send_coin(coin: Coin, to: &Addr) -> CosmosMsg {
  CosmosMsg::Bank(BankMsg::Send {
    to_address: to.to_string(),
    amount: vec![coin],
  })
}

/// Create bank send message for multiple coins to a single address.
pub fn send_coins(coins: impl Into<Vec<Coin>>, to: &Addr) -> CosmosMsg {
  CosmosMsg::Bank(BankMsg::Send {
    to_address: to.to_string(),
    amount: coins.into(),
  })
}

/// Bank message input or output. See [protobuf definition](https://github.com/cosmos/cosmos-sdk/blob/v0.40.0/proto/cosmos/bank/v1beta1/bank.proto#L27).
#[cw_serde]
pub struct BankMsgIo {
  address: Addr,
  coins: Vec<Coin>,
}

impl BankMsgIo {
  pub fn new(address: Addr, coins: Vec<Coin>) -> Self {
    Self { address, coins }
  }
}

/// Multi-send bank message. See [protobuf definition](https://github.com/cosmos/cosmos-sdk/blob/v0.40.0/proto/cosmos/bank/v1beta1/tx.proto#L33).
#[cw_serde]
pub struct BankMsgMultiSend {
  pub inputs: Vec<BankMsgIo>,
  pub outputs: Vec<BankMsgIo>,
}

/// Create bank multi-send message for multiple coins to multiple addresses. Not supported
/// natively in `cosmwasm_std`; encodes a `/cosmos.bank.v1beta1.MsgMultiSend` as
/// [`BankMsgMultiSend`] using [`CosmosMsg::Any`]`.
#[cfg(feature = "cosmwasm_2_0")]
pub fn send_coins_many(
  coins: &CoinSet,
  from: &Addr,
  to: Vec<(&Addr, CoinSet)>,
) -> CoinResult<CosmosMsg> {
  let mut rem: CoinSet = coins.clone();
  let mut outputs: Vec<BankMsgIo> = Vec::with_capacity(to.len());
  for (addr, out_coins) in to.into_iter() {
    for coin in out_coins.iter_coins() {
      rem
        .try_minus_mut(&coin)
        .map_err(|_| CoinError::Insufficient {
          expected: coin.to_string(),
        })?;
      outputs.push(BankMsgIo {
        address: addr.clone(),
        coins: vec![coin],
      });
    }
  }
  rem.expect_none().map_err(|_| CoinError::IoMismatch {})?;
  let inputs: Vec<BankMsgIo> = vec![BankMsgIo {
    address: from.clone(),
    coins: coins.into(),
  }];
  Ok(CosmosMsg::Any(AnyMsg {
    type_url: "/cosmos.bank.v1beta1.MsgMultiSend".to_string(),
    value: to_json_binary(&BankMsgMultiSend { inputs, outputs }).map_err(|err| {
      CoinError::Unexpected {
        msg: format!("unable to serialize BankMsgMultiSend: {}", err),
      }
    })?,
  }))
}

#[cfg(not(feature = "cosmwasm_2_0"))]
pub fn send_coins_many(
  _coins: &CoinSet,
  _from: &Addr,
  to: Vec<(&Addr, CoinSet)>,
) -> CoinResult<CosmosMsg> {
  to.into_iter()
    .map(|(addr, out_coins)| send_coins(out_coins.into(), addr))
    .collect()
}
