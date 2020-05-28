#![cfg_attr(not(feature = "std"), no_std)]
use frame_support::{debug::info, decl_error, decl_event, decl_module, decl_storage, dispatch};
use frame_system::{self as system, ensure_signed};
use sp_std::{prelude::Vec, vec};

const CHANGE_WAITING_BLOCKS: u32 = 10;

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
        LastComfirmedHeader get(fn last_comfirm_header): Option<types::EthHeader>;
        SubmitHeadersMap get(fn submit_headers_map): map hasher(blake2_128_concat) types::EthereumBlockHeightType => Vec<types::RelayHeader::<T::AccountId, T::BlockNumber>>;
        SubmitHeaders get(fn submit_headers): Vec<types::EthereumBlockHeightType>;
        NextSamplingHeader get(fn next_sampling_header): Option<types::EthereumBlockHeightType>;
    }
}

decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as system::Trait>::AccountId,
    {
        UpdateLastComfrimedBlock(u32, AccountId),
        SubmitHeader(u32, AccountId),
    }
);

decl_error! {
    pub enum Error for Module<T: Trait> {
        HeaderInvalid,
        SubmitHeaderAlreadyComfirmed,
        SubmitHeaderNotInSamplingList,
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        type Error = Error<T>;
        fn deposit_event() = default;

        #[weight = 0]
        pub fn set_last_comfirm_header(origin, header: types::EthHeader) -> dispatch::DispatchResult {
            info!(target: "relay", "last comfirm header: {:?}", header);
            if header.lie > 0 {
                Err(<Error<T>>::HeaderInvalid)?;
            }
            let who = ensure_signed(origin)?;
            let block_height = header.block_height;

            LastComfirmedHeader::put(header);

            Self::deposit_event(RawEvent::UpdateLastComfrimedBlock(block_height, who));
            Ok(())
        }

        #[weight = 0]
        pub fn submit(origin, header: types::EthHeader) -> dispatch::DispatchResult {
            info!(target: "relay", "submit header: {:?}", header);
            if header.lie > 0 {
                Err(<Error<T>>::HeaderInvalid)?;
            }
            let current_block = <frame_system::Module<T>>::block_number();

            if let Some(next) = NextSamplingHeader::get(){
                if header.block_height != next {
                    if <SubmitHeadersMap<T>>::contains_key(header.block_height) {
                        if current_block > <SubmitHeadersMap<T>>::get(header.block_height)[0].challenge_block_height {
                            Err(<Error<T>>::SubmitHeaderAlreadyComfirmed)?;
                        }
                    } else {
                        Err(<Error<T>>::SubmitHeaderNotInSamplingList)?;
                    }
                }
            }

            let who = ensure_signed(origin)?;
            let block_height: types::EthereumBlockHeightType = header.block_height;
            let mut submissions;

            if <SubmitHeadersMap<T>>::contains_key(block_height) {
                submissions = <SubmitHeadersMap<T>>::get(block_height);
                let mut is_exist = false;
                for rh in submissions.iter_mut() {
                    if header == rh.header {
                        rh.relayers.push(who.clone());
                        is_exist = true;
                        break;
                    }
                }
                if !is_exist {
                    let relay_header = types::RelayHeader::<<T as system::Trait>::AccountId, <T as system::Trait>::BlockNumber> {
                        header: header,
                        relay_position: current_block,
                        challenge_block_height: current_block + CHANGE_WAITING_BLOCKS.into(),
                        relayers: vec![who.clone()]
                    };
                    submissions.push(relay_header);
                }
            } else {
                let current_block = <frame_system::Module<T>>::block_number();
                let relay_header = types::RelayHeader::<<T as system::Trait>::AccountId, <T as system::Trait>::BlockNumber> {
                    header: header,
                    relay_position: current_block.into(),
                    challenge_block_height: current_block + CHANGE_WAITING_BLOCKS.into(),
                    relayers: vec![who.clone()]
                };
                submissions = vec![relay_header];
                SubmitHeaders::mutate(|v| v.push(block_height));
                let last_comfirm_header = if let Some(h) =LastComfirmedHeader::get() {h.block_height} else {0 as types::EthereumBlockHeightType};
                let next_sampling_block_height = (last_comfirm_header + block_height) / 2;
                NextSamplingHeader::put(next_sampling_block_height);
            }
            <SubmitHeadersMap<T>>::insert(block_height, submissions);

            Self::deposit_event(RawEvent::SubmitHeader(block_height, who));
            Ok(())
        }

        fn offchain_worker(block: T::BlockNumber) {
            let submit_headers  = SubmitHeaders::get();
            let mut honest_relayers = Vec::new();
            if let Some(last) = submit_headers.last() {
                let submissions = <SubmitHeadersMap<T>>::get(last);
                if submissions.len() == 1 && submissions[0].challenge_block_height < block {
                    honest_relayers = submissions[0].relayers.clone();
                    LastComfirmedHeader::put(submissions[0].header.clone());
                    Self::deposit_event(
                        RawEvent::UpdateLastComfrimedBlock(submissions[0].header.block_height,
                                                           honest_relayers[0].clone()));
                }

            }
            if honest_relayers.len() > 1 {
                // TODO: slash and reward here
                info!("Honest Relayers: {:?}", honest_relayers);
            }

            SubmitHeaders::mutate(|_| Vec::<types::EthereumBlockHeightType>::new());
            NextSamplingHeader::mutate(|_| None as Option<types::EthereumBlockHeightType>);
        }
    }
}
