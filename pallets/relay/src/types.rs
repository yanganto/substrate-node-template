use codec::{Decode, Encode};
use sp_runtime::RuntimeDebug;
use sp_std::vec;

pub type EthereumBlockHeightType = u32;

#[derive(Encode, Decode, Default, Clone, PartialEq, RuntimeDebug)]
pub struct EthHeader {
    /// lie: 0 is honest, there are a lot of lie headers but only one honest header
    pub lie: u32,
    pub block_height: EthereumBlockHeightType,
}

#[derive(Encode, Decode, Default, Clone, PartialEq, RuntimeDebug)]
pub struct RelayHeader<AccountId, BlockNumber> {
    pub header: EthHeader,
    pub relay_position: BlockNumber,
    pub challenge_block_height: BlockNumber,
    pub relayers: vec::Vec<AccountId>,
}
