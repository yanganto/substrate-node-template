use codec::{Decode, Encode};
use sp_runtime::RuntimeDebug;
use sp_std::vec;

pub type EthereumBlockHeightType = u32;
pub type SubmitRound = u32;

/// The Ethereum header just mimic the real ether header
#[derive(Encode, Decode, Default, Copy, Clone, PartialEq, RuntimeDebug)]
pub struct EthHeader {
    /// lie: 0 is honest, there are a lot of lie headers but only one honest header
    pub lie: u32,
    pub block_height: EthereumBlockHeightType,
}

/// Here we use (header.block_height, header.lie) as Proposal ID to store
/// In real scenario, we should use (header.block_height, header.hash) as Proposal ID
#[derive(Encode, Decode, Default, Clone, PartialEq, RuntimeDebug)]
pub struct Proposal<AccountId> {
    /// The raw ethereum header
    pub headers: vec::Vec<EthHeader>,
    pub round: u32,

    /// Record the relayer submit the same blocks
    pub relayer: AccountId,

    /// Here we use storage the block heigh and lie for the last sample block of previous round,
    /// but in production, this will be block heigh and hash
    pub extend_from: Option<(EthereumBlockHeightType, u32)>,
}
