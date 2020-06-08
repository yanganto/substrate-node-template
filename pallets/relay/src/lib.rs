#![cfg_attr(not(feature = "std"), no_std)]
use frame_support::{debug::info, decl_error, decl_event, decl_module, decl_storage, dispatch};
use frame_system::{self as system, ensure_signed};
use sp_std::prelude::Vec;

const CHANGE_WAITING_BLOCKS: u32 = 50;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

mod types;

pub trait Trait: system::Trait + sample::Trait {
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

const fn num_bits<T>() -> usize {
    sp_std::mem::size_of::<T>() * 8
}

decl_storage! {
    trait Store for Module<T: Trait> as Relay {
        /// This the `G` for as README of relayer-game
        LastConfirmedHeader get(fn last_comfirm_header): Option<types::EthHeader>;

        /// The blocks confrimed
        ConfirmedBlocks get(fn confrimed_blocks): map hasher(blake2_128_concat) types::EthereumBlockHeightType => types::EthHeader;

        /// use the last round header.block_height as Proposal ID to store
        ProposalMap get(fn proposal_map): map hasher(blake2_128_concat) types::EthereumBlockHeightType => Vec<types::Proposal::<T::AccountId>>;

        /// use the block number of challenge time as key to last round header.block_height and round
        ChallengeTimes get(fn challenge_time): map hasher(blake2_128_concat) T::BlockNumber =>  Vec<(types::EthereumBlockHeightType, types::SubmitRound)>;
    }
}

decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as system::Trait>::AccountId,
    {
        UpdateLastConfrimedBlock(types::EthereumBlockHeightType, AccountId),
        SubmitHeaders(types::EthereumBlockHeightType, types::SubmitRound),
    }
);

decl_error! {
    pub enum Error for Module<T: Trait> {
        HeaderInvalid,
        NotExtendFromError,
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

            LastConfirmedHeader::put(header);

            Self::deposit_event(RawEvent::UpdateLastConfrimedBlock(block_height, who));
            Ok(())
        }

        #[weight = 0]
        pub fn submit(origin, headers: Vec<types::EthHeader>) -> dispatch::DispatchResult {
            info!(target: "relay", "headers proposal: {:?}", headers);

            let who = ensure_signed(origin)?;

            // log_2 (header_length + 1)
            let current_round = num_bits::<i32>() as u32 - (headers.len() + 1).leading_zeros() - 1;
            info!(target: "relay", "submit round: {}, headers : {:?}", current_round, headers);

            // If submission not at first round, the submission should extend from previous
            // submission
            if current_round > 1 {
                let last_sample_of_prvious_proposal = headers.len() - 2usize.pow(current_round -1) - 1;
                let prvious_round = current_round - 1;
                let mut is_extend_from = false;
                for p in <ProposalMap<T>>::get(headers[last_sample_of_prvious_proposal].block_height) {
                    if p.round == prvious_round  {
                        let mut all_header_equal = true;
                        for (i, h) in p.headers.into_iter().enumerate() {
                            if h != headers[i] {
                                all_header_equal = false;
                                break;
                            }
                        }
                        if all_header_equal {
                            is_extend_from = true;
                            break;
                        }
                    }
                }
                if ! is_extend_from {
                    Err(<Error<T>>::NotExtendFromError)?;
                }
            }

            if headers.len() == 0 {
                Err(<Error<T>>::HeaderInvalid)?;
            }
            // Validate Blocks
            // NOTE In production, the handler should check this
            for header in &headers {
                if header.lie > 0 {
                    Err(<Error<T>>::HeaderInvalid)?;
                }
            }

            let last_header = headers.last().unwrap();
            if <ProposalMap<T>>::get(last_header.block_height).len() == 0 {
                let challenge_end_block = <frame_system::Module<T>>::block_number() + CHANGE_WAITING_BLOCKS.into();
                <ChallengeTimes<T>>::mutate(challenge_end_block, |v| v.push((last_header.block_height, current_round)));
            }
            <ProposalMap<T>>::mutate(last_header.block_height, |v| v.push(types::Proposal{
                round: current_round,
                relayer: who,
                headers,
            }));

            Ok(())
        }

        // TODO: this offchain worker is a POC, it is not send data back on chain
        // in production the mutation of data should be send back on chain
        fn offchain_worker(block: T::BlockNumber) {

        }
    }
}
impl<T: Trait> Module<T> {}
