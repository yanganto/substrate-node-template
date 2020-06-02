#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{decl_error, decl_event, decl_module, decl_storage, dispatch};
use frame_system::{self as system, ensure_signed};
use sp_std::prelude::Vec;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub trait Trait: system::Trait {
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

decl_storage! {
    trait Store for Module<T: Trait> as SampleModule {
        ConfirmBlocks get(fn confirm_blocks): Vec<u32>;
    }
}

decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as system::Trait>::AccountId,
    {
        BlockConfirmed(u32, AccountId),
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
            ConfirmBlocks::mutate(|v| v.push(block_height));
            Self::deposit_event(RawEvent::BlockConfirmed(block_height, who));
            Ok(())
        }
    }
}
