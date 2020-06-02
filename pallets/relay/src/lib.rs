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
        LastConfirmedHeader get(fn last_comfirm_header): Option<types::EthHeader>;

        /// The blocks confrimed
        ConfirmedBlocks get(fn confrimed_blocks): map hasher(blake2_128_concat) types::EthereumBlockHeightType => types::EthHeader;

        /// Here we use (header.block_height, header.lie) as Proposal ID to store
        /// In real scenario, we should use (header.block_height, header.hash) as Proposal ID
        ProposalMap get(fn proposal_map): map hasher(blake2_128_concat) (types::EthereumBlockHeightType, u32) => types::Proposal::<T::AccountId, T::BlockNumber>;

        /// Here store all the proposals: key is level, value is Proposal ID and challenge time
        ProposalLevelMap get(fn proposal_ids_by_level): map hasher(blake2_128_concat) u32 => Vec<(types::EthereumBlockHeightType, u32, T::BlockNumber)> ;

        /// Store the highest proposal level
        HighestLevel get(fn highest_level): u32;

        SubmitHeaders get(fn submit_headers): Vec<types::EthereumBlockHeightType>;
    }
}

decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as system::Trait>::AccountId,
    {
        UpdateLastConfrimedBlock(u32, AccountId),
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

            LastConfirmedHeader::put(header);

            Self::deposit_event(RawEvent::UpdateLastConfrimedBlock(block_height, who));
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
                if proposal.against.is_none() {
                    Err(<Error<T>>::AgainstAbsent)?;
                }
                agree = Some(proposal.header.block_height);
                let mut against_proposal_id = proposal.against;
                while against_proposal_id.is_some(){
                    let against_proposal = <ProposalMap<T>>::get(against_proposal_id.unwrap());
                    if  against_proposal.header.block_height < proposal.header.block_height {
                        against_proposal_id = against_proposal.take_over;
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
                let mut take_over_proposal_id = proposal.take_over;
                while take_over_proposal_id.is_some() {
                    let take_over_proposal = <ProposalMap<T>>::get(take_over_proposal_id.unwrap());
                    if  take_over_proposal.header.block_height > proposal.header.block_height {
                        take_over_proposal_id = take_over_proposal.take_over;
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
                    agree = if let Some(h) = LastConfirmedHeader::get() {
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
            <ProposalLevelMap<T>>::mutate(p.level, |v| v.push(( p.header.block_height, p.header.lie, p.challenge_block_height)));
            <ProposalMap<T>>::insert((p.header.block_height, p.header.lie), p);
            if proposal.level > HighestLevel::get() {
                HighestLevel::put(proposal.level)
            }

            if disagree.is_some(){
                <sample::Call<T>>::gen_sampling_blocks(disagree.unwrap(), agree.unwrap());
            }
            Ok(())
        }

        // TODO: this offchain worker is a POC, it is not send data back on chain
        // in production the mutation of data should be send back on chain
        fn offchain_worker(block: T::BlockNumber) {
            let highest_level = HighestLevel::get();
            let proposals = <ProposalLevelMap<T>>::get(highest_level);

            let mut over_challenge_time_list = Vec::<(types::EthereumBlockHeightType, u32)>::new();
            let mut in_challenge_time_list = Vec::<(types::EthereumBlockHeightType, u32,T::BlockNumber)>::new();

            if in_challenge_time_list.len() == 0 && highest_level > 0 {
                HighestLevel::put(highest_level-1);
            }

            for p_info in proposals {
                if block > p_info.2 {
                    over_challenge_time_list.push((p_info.0, p_info.1));
                } else {
                    in_challenge_time_list.push(p_info);
                }
            }
            <ProposalLevelMap<T>>::mutate( highest_level, |_| in_challenge_time_list);

            for proposal_id in over_challenge_time_list {
                let mut p = Some(proposal_id);
                while p.is_some() {
                    let proposal = <ProposalMap<T>>::take(p.unwrap());

                    // NOTE In production check block integrality is not checking the lie flag
                    // NOTE In production, please try to use block_height + 1 and block_height - 1 to
                    // verify the block, althought over challenge time, we still need to be carefure to
                    // verify the blocks.
                    if proposal.header.lie == 0  && !ConfirmedBlocks::contains_key(proposal.header.block_height) {
                        <sample::Call<T>>::confirm(proposal.header.block_height);
                        ConfirmedBlocks::insert(proposal.header.block_height, proposal.header);
                    }
                    p = proposal.take_over;
                }
            }

        }
    }
}
impl<T: Trait> Module<T> {}
