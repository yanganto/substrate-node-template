#![cfg_attr(not(feature = "std"), no_std)]
use frame_support::{debug::info, decl_error, decl_event, decl_module, decl_storage, dispatch};
use frame_system::{self as system, ensure_signed};
use sp_std::vec;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

mod types;

pub trait Trait: system::Trait {
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

decl_storage! {
    trait Store for Module<T: Trait> as Relay {
        pub LastComfirmedHeader get(fn last_comfirm_header): Option<types::RelayHeader::<T::AccountId, T::BlockNumber>>;
    }
}

decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as system::Trait>::AccountId,
    {
        UpdateLastComfrimedBlock(u32, AccountId),
    }
);

decl_error! {
    pub enum Error for Module<T: Trait> {
        HeaderInvalid,
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        type Error = Error<T>;
        fn deposit_event() = default;

        #[weight = 0]
        pub fn set_last_comfirm_header(origin, header: types::EthHeader) -> dispatch::DispatchResult {
            info!(target: "relay", "header: {:?}", header);
            if header.lie > 0 {
                Err(<Error<T>>::HeaderInvalid)?;
            }
            let who = ensure_signed(origin)?;
            let block_height = header.block_height;
            let relay_header = types::RelayHeader::<<T as system::Trait>::AccountId, <T as system::Trait>::BlockNumber> {
                header: header,
                relay_position: 0u32.into(),
                challenge_block_height: 0u32.into(),
                relayer: vec![who.clone()]
            };
            <LastComfirmedHeader<T>>::put(relay_header);
            Self::deposit_event(RawEvent::UpdateLastComfrimedBlock(block_height, who));
            Ok(())
        }
    }
}
