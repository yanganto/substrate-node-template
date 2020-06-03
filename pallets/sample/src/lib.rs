#![cfg_attr(not(feature = "std"), no_std)]
use crate::sp_api_hidden_includes_decl_storage::hidden_include::traits::Randomness;
use frame_support::{decl_error, decl_event, decl_module, decl_storage, dispatch, traits::Get};
use frame_system::{self as system, ensure_signed};
use sp_runtime::traits::{BlakeTwo256, Hash};
use sp_runtime::RandomNumberGenerator;
use sp_std::prelude::Vec;
use sp_std::vec;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub type EthereumBlockHeightType = u32;

pub enum ChainType {
    Normal,
    POW,
}

pub trait Trait: system::Trait {
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;

    /// The algorithm of sampling depends on the consensus mechanism of the target chain
    type ChainType: Get<ChainType>;

    /// If the sample block in the CONFIRMED_BLOCK_ATTRACT_RANGE range of confirm block, the sample
    /// block will change into the block near by the comfirm block
    type ConfrimBlockAttractRange: Get<EthereumBlockHeightType>;
}

decl_storage! {
    trait Store for Module<T: Trait> as TemplateModule {
        ConfirmedBlocks get(fn confirmed_blocks): Vec<EthereumBlockHeightType>;
        /// the key is (disagree position, agree position)
        SamplingBlocksMap get(fn sampling_blocks_map): map hasher(blake2_128_concat) (EthereumBlockHeightType, EthereumBlockHeightType) => EthereumBlockHeightType;
        pub SamplingBlocks get(fn sampling_blocks): Vec<EthereumBlockHeightType>;
    }
}

decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as system::Trait>::AccountId,
    {
        BlockConfirmed(EthereumBlockHeightType, AccountId),
    }
);

decl_error! {
    pub enum Error for Module<T: Trait> {
        NoneValue,
        StorageOverflow,
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        type Error = Error<T>;

        fn deposit_event() = default;

        #[weight = 0]
        pub fn confirm(origin, block_height: u32) -> dispatch::DispatchResult {
            let who = ensure_signed(origin)?;
            ConfirmedBlocks::mutate(|v| v.push(block_height));
            Self::deposit_event(RawEvent::BlockConfirmed(block_height, who));
            Ok(())
        }

        #[weight = 0]
        pub fn gen_sampling_blocks(_origin, disagree: EthereumBlockHeightType, agree: EthereumBlockHeightType) -> dispatch::DispatchResult {
            if !SamplingBlocksMap::contains_key((disagree, agree)) {
                let r = <pallet_randomness_collective_flip::Module<T>>::random_seed();
                let raw_sample_position = match T::ChainType::get() {
                    ChainType::POW => Self::get_sample_tail_more_from_random_number(disagree, agree, r),
                    _ => Self::get_sample_from_random_number(disagree, agree, r),
                };
                let sample_position = Self::handle_confirm_blocks_affinity(disagree, agree, raw_sample_position);
                SamplingBlocksMap::insert((disagree, agree), sample_position);
                SamplingBlocks::mutate(|v| v.push(sample_position));
            }
            Ok(())
        }
    }
}

impl<T: Trait> Module<T> {
    fn handle_confirm_blocks_affinity(
        disagree: EthereumBlockHeightType,
        agree: EthereumBlockHeightType,
        raw_sample_position: EthereumBlockHeightType,
    ) -> EthereumBlockHeightType {
        let confirmed_blocks = ConfirmedBlocks::get();
        let &lower_bondary = vec![
            disagree,
            agree,
            raw_sample_position - T::ConfrimBlockAttractRange::get(),
        ]
        .iter()
        .filter(|i| **i < raw_sample_position)
        .max()
        .unwrap_or(&raw_sample_position);
        for i in lower_bondary..raw_sample_position {
            if confirmed_blocks.contains(&i) {
                return i + 1;
            }
        }
        let &higher_bondary = vec![
            disagree,
            agree,
            raw_sample_position + T::ConfrimBlockAttractRange::get(),
        ]
        .iter()
        .filter(|i| **i > raw_sample_position)
        .min()
        .unwrap_or(&raw_sample_position);
        for i in raw_sample_position..higher_bondary {
            if confirmed_blocks.contains(&i) {
                return i - 1;
            }
        }
        raw_sample_position
    }
    /// This is the basic sampling function
    fn get_sample_from_random_number(
        e1: EthereumBlockHeightType,
        e2: EthereumBlockHeightType,
        r: <T as frame_system::Trait>::Hash,
    ) -> EthereumBlockHeightType {
        let random_seed = BlakeTwo256::hash(r.as_ref());
        let mut rng = <RandomNumberGenerator<BlakeTwo256>>::new(random_seed);
        let eth_range: u32;
        let base: u32;
        if e2 > e1 {
            eth_range = e2 - e1 - 2;
            base = e1 + 1;
        } else {
            eth_range = e1 - e2 - 2;
            base = e2 + 1;
        };
        base + rng.pick_u32(eth_range)
    }
    /// This function is for PoW chain, sample on tail part more
    fn get_sample_tail_more_from_random_number(
        e1: EthereumBlockHeightType,
        e2: EthereumBlockHeightType,
        r: <T as frame_system::Trait>::Hash,
    ) -> EthereumBlockHeightType {
        let random_seed = BlakeTwo256::hash(r.as_ref());
        let mut rng = <RandomNumberGenerator<BlakeTwo256>>::new(random_seed);
        let eth_range: f32;
        let base: u32;
        if e2 > e1 {
            eth_range = (e2 - e1 - 2) as f32;
            base = e1 + 1;
        } else {
            eth_range = (e1 - e2 - 2) as f32;
            base = e2 + 1;
        };

        // If std use sin function to sample,
        // else just use random number generator
        //
        // r is in (0, PI/2)
        #[cfg(feature = "std")]
        let r = rng.pick_u32(2147483647) as f32 / 1367130550.5162435f32;
        #[cfg(feature = "std")]
        let ext_range = (eth_range * r.sin()) as u32;
        #[cfg(not(feature = "std"))]
        let ext_range = rng.pick_u32(eth_range as u32);

        base + ext_range
    }
}
