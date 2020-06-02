use codec::{Decode, Encode};
use sp_runtime::RuntimeDebug;
use sp_std::vec;

pub type EthereumBlockHeightType = u32;

/// The Ethereum header just mimic the real ether header
#[derive(Encode, Decode, Default, Clone, PartialEq, RuntimeDebug)]
pub struct EthHeader {
    /// lie: 0 is honest, there are a lot of lie headers but only one honest header
    pub lie: u32,
    pub block_height: EthereumBlockHeightType,
}

/// Here we use (header.block_height, header.lie) as Proposal ID to store
/// In real scenario, we should use (header.block_height, header.hash) as Proposal ID
#[derive(Encode, Decode, Default, Clone, PartialEq, RuntimeDebug)]
pub struct Proposal<AccountId, BlockNumber> {
    /// The raw ethereum header
    pub header: EthHeader,

    /// Against Proposal we use (header.block_height, header.lie) as Proposal ID
    pub against: Option<(EthereumBlockHeightType, u32)>,

    /// Take-Over Proposal we use (header.block_height, header.lie) as Proposal ID
    pub take_over: Option<(EthereumBlockHeightType, u32)>,

    /// The proposal level
    pub level: u32,

    /// The last block for challenge time
    pub challenge_block_height: BlockNumber,

    /// Record the relayer submit the same blocks
    pub relayers: vec::Vec<AccountId>,
}
