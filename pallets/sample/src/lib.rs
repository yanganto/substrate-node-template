#![cfg_attr(not(feature = "std"), no_std)]

use crate::sp_api_hidden_includes_decl_storage::hidden_include::traits::Randomness;
use frame_support::{debug::info, decl_error, decl_event, decl_module, decl_storage, dispatch};
use frame_system::{self as system, ensure_signed};
use sp_std::prelude::Vec;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

type EthereumBlockHeightType = u32;

pub trait Trait: system::Trait {
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
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
                // let r = <pallet_randomness_collective_flip::Module<T>>::random(b"sample");
                info!(target: "sample", "ANT-DEBUG: {:?}", r );
                let sample_position = (disagree + agree) / 2;
                SamplingBlocks::insert((disagree, agree), sample_position);
            }
            Ok(())
        }
    }
}
