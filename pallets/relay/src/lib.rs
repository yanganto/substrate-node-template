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

decl_storage! {
    trait Store for Module<T: Trait> as Relay {
        /// This the `G` for as README of relayer-game
        LastComfirmedHeader get(fn last_comfirm_header): Option<types::EthHeader>;

        /// Here we use (header.block_height, header.lie) as Proposal ID to store
        /// In real scenario, we should use (header.block_height, header.hash) as Proposal ID
        ProposalMap get(fn proposal_map): map hasher(blake2_128_concat) (types::EthereumBlockHeightType, u32) => types::Proposal::<T::AccountId, T::BlockNumber>;

        SubmitHeaders get(fn submit_headers): Vec<types::EthereumBlockHeightType>;
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
        ProposalLevelInvalid,
        AgainstAbsent,
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
        pub fn submit(_origin, proposal: types::Proposal::<T::AccountId, T::BlockNumber>) -> dispatch::DispatchResult {
            info!(target: "relay", "submit proposal: {:?}", proposal);

            // NOTE In production, the handler should check this
            // if proposal.header.lie > 0 {
            //     Err(<Error<T>>::HeaderInvalid)?;
            // }

            let proposal_level_correct = if let Some(take_over_proposal_id) = proposal.take_over {
                let take_over_proposal = <ProposalMap<T>>::get(take_over_proposal_id);
                take_over_proposal.level + 1 == proposal.level
            } else {
                1 == proposal.level
            };

            if !proposal_level_correct {
                Err(<Error<T>>::ProposalLevelInvalid)?;
            }

            // find out the agree position and the disagree position
            let mut agree: Option<types::EthereumBlockHeightType> = None;
            let mut disagree: Option<types::EthereumBlockHeightType> = None;
            let submit_headers = SubmitHeaders::get();

            if submit_headers.contains(&proposal.header.block_height) {
                agree = Some(proposal.header.block_height);
                let mut against_proposal_id = proposal.against;
                while against_proposal_id.is_some(){
                    let against_proposal = <ProposalMap<T>>::get(against_proposal_id.unwrap());
                    if  against_proposal.header.block_height < proposal.header.block_height {
                        continue;
                    }

                    if disagree.is_none() {
                        disagree = Some(against_proposal.header.block_height);
                    } else if let Some(disagree_block_height) = disagree {
                        if against_proposal.header.block_height < disagree_block_height {
                            disagree = Some(against_proposal.header.block_height);
                        }
                    }
                    against_proposal_id = against_proposal.take_over;
                }

            } else {
                if proposal.against.is_none() {
                    Err(<Error<T>>::AgainstAbsent)?;
                }
                let mut take_over_proposal_id = proposal.take_over;
                while take_over_proposal_id.is_some() {
                    let take_over_proposal = <ProposalMap<T>>::get(take_over_proposal_id.unwrap());
                    if  take_over_proposal.header.block_height > proposal.header.block_height {
                        continue;
                    }
                    if agree.is_none() {
                        agree = Some(take_over_proposal.header.block_height);
                    } else if let Some(agree_block_height) = agree {
                        if take_over_proposal.header.block_height > agree_block_height {
                            agree = Some(take_over_proposal.header.block_height);
                        }
                    }
                    take_over_proposal_id = take_over_proposal.take_over;
                }
                if agree.is_none() {
                    // use genesis or last comfirmed block for the agree point
                    agree = if let Some(h) = LastComfirmedHeader::get() {
                        Some(h.block_height)
                    }else {
                        Some(0)
                    };
                }

                let against_proposal = <ProposalMap<T>>::get(proposal.against.unwrap());
                disagree = Some(against_proposal.header.block_height);
            }

            let current_block = <frame_system::Module<T>>::block_number();
            let mut p = proposal.clone();
            p.challenge_block_height = current_block + CHANGE_WAITING_BLOCKS.into();
            <ProposalMap<T>>::insert((p.header.block_height, p.header.lie), p);

            if disagree.is_some(){
                <sample::Call<T>>::gen_sampling_blocks(disagree.unwrap(), agree.unwrap());
            }
            Ok(())
        }

        fn offchain_worker(_block: T::BlockNumber) {
        }
    }
}
impl<T: Trait> Module<T> {}
