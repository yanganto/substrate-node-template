#![cfg_attr(not(feature = "std"), no_std)]

use crate::sp_api_hidden_includes_decl_storage::hidden_include::traits::Randomness;
use frame_support::{decl_error, decl_event, decl_module, decl_storage, dispatch, traits::Get};
use frame_system::{self as system, ensure_signed};
use sp_runtime::traits::{BlakeTwo256, Hash};
use sp_runtime::RandomNumberGenerator;
use sp_std::prelude::Vec;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

type EthereumBlockHeightType = u32;

pub enum ChainType {
    Normal,
    POW,
}

pub trait Trait: system::Trait {
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
    type ChainType: Get<ChainType>;
}

decl_storage! {
    trait Store for Module<T: Trait> as TemplateModule {
        ConfirmedBlocks get(fn confirmed_blocks): Vec<EthereumBlockHeightType>;
        /// the key is (disagree position, agree position)
        SamplingBlocks get(fn sampling_blocks): map hasher(blake2_128_concat) (EthereumBlockHeightType, EthereumBlockHeightType) => EthereumBlockHeightType;
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
            if !SamplingBlocks::contains_key((disagree, agree)) {
                let r = <pallet_randomness_collective_flip::Module<T>>::random_seed();
                let sample_position = match T::ChainType::get() {
                    ChainType::POW => Self::get_sample_tail_more_from_random_number(disagree, agree, r),
                    _ => Self::get_sample_from_random_number(disagree, agree, r),
                };
                // TODO: take the confirmed blocks into consideration
                SamplingBlocks::insert((disagree, agree), sample_position);
            }
            Ok(())
        }
    }
}

impl<T: Trait> Module<T> {
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
        let eth_range: u32;
        let base: u32;
        if e2 > e1 {
            eth_range = e2 - e1 - 2;
            base = e1 + 1;
        } else {
            eth_range = e1 - e2 - 2;
            base = e2 + 1;
        };
        // TODO: Use a better sampling equation for POW chain
        base + rng.pick_u32(eth_range)
    }
}
