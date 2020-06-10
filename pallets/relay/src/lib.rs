#![cfg_attr(not(feature = "std"), no_std)]
use frame_support::{debug::info, decl_error, decl_event, decl_module, decl_storage, dispatch};
use frame_system::{self as system, ensure_signed};
use sp_std::{prelude::Vec, vec};

const CHANGE_WAITING_BLOCKS: u32 = 50;
const BOND_VALUE: u32 = 100;

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

        /// The allow samples for each game, the block height of first submit is the key
        Samples get(fn get_samples): map hasher(blake2_128_concat) types::EthereumBlockHeightType => Vec<types::EthereumBlockHeightType>;
    }
}

decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as system::Trait>::AccountId,
    {
        UpdateLastConfrimedBlock(types::EthereumBlockHeightType, AccountId),

        /// Publish event with first block height, last block height, and round
        SubmitHeaders(
            types::EthereumBlockHeightType,
            types::EthereumBlockHeightType,
            types::SubmitRound,
        ),
    }
);

decl_error! {
    pub enum Error for Module<T: Trait> {
        HeaderInvalid,
        NotExtendFromError,
        NotComplyWithSamples,
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

            let current_round = Self::get_current_round_from_submit_length(headers.len());
            info!(target: "relay", "submit round: {}, headers : {:?}", current_round, headers);

            let mut last_block_hash_of_previous_round: Option<(types::EthereumBlockHeightType, u32)> = None;

            if headers.len() == 0 {
                Err(<Error<T>>::HeaderInvalid)?;
            }

            // Validate Blocks
            // NOTE In production, the handler should check this
            // for header in &headers {
            //     if header.lie > 0 {
            //         Err(<Error<T>>::HeaderInvalid)?;
            //     }
            // }

            // After validated, the headers will be shrinked for headers in current round only,
            // so the mutability chganged
            let mut headers = headers;

            // If submission not at first round, the submission should extend from previous
            // submission
            if current_round > 1 {
                let samples = Samples::get(headers[0].block_height);
                if samples.len() !=  headers.len() {
                    Err(<Error<T>>::NotComplyWithSamples)?;
                }
                for (idx, s) in samples.into_iter().enumerate() {
                    if s != headers[idx].block_height {
                        Err(<Error<T>>::NotComplyWithSamples)?;
                    }
                }

                let last_sample_of_prvious_proposal = headers.len() - 2usize.pow(current_round -2) - 1;
                let prvious_round = current_round - 1;

                // Check the proposal is extended from some proposal before
                // The "Cut in line" scenario is not allowed in this implementation
                let mut is_extend_from = false;
                for p in <ProposalMap<T>>::get(headers[last_sample_of_prvious_proposal].block_height) {
                    if p.round == prvious_round  {
                        let last_header = p.headers.last().unwrap();
                        last_block_hash_of_previous_round = Some((last_header.block_height, last_header.lie));
                        let num_of_samples_in_round = p.headers.len();

                        let mut all_header_equal = true;
                        for (i, h) in p.headers.into_iter().enumerate() {
                            if h != headers[i] {
                                all_header_equal = false;
                                break;
                            }
                        }

                        if all_header_equal {
                            is_extend_from = true;

                            // save sample headers of the current rount only
                            headers = headers[num_of_samples_in_round ..].to_vec();
                            break;
                        }
                    }
                }
                if ! is_extend_from {
                    Err(<Error<T>>::NotExtendFromError)?;
                }
            } else if Samples::get(headers[0].block_height).len() == 0{
                Self::set_samples(&vec![headers[0].block_height]);
            }

            let first_header_block_height = headers.first().unwrap().block_height;
            let last_header_block_height = headers.last().unwrap().block_height;

            // update the challenge time when the first submit of the round comes in
            if <ProposalMap<T>>::get(last_header_block_height).len() == 0 {
                let challenge_end_block = <frame_system::Module<T>>::block_number() + CHANGE_WAITING_BLOCKS.into();
                <ChallengeTimes<T>>::mutate(challenge_end_block, |v| v.push((last_header_block_height, current_round)));
            }

            <ProposalMap<T>>::mutate(last_header_block_height, |v| v.push(types::Proposal{
                round: current_round,
                relayer: who,
                extend_from: last_block_hash_of_previous_round,
                headers,
            }));

            Self::deposit_event(RawEvent::SubmitHeaders(first_header_block_height, last_header_block_height, current_round));

            Ok(())
        }

        // TODO: this offchain worker is a POC, it is not send data back on chain
        // in production the mutation of data should be send back on chain
        fn offchain_worker(block: T::BlockNumber) {
            let proposal_queries = <ChallengeTimes<T>>::take(block);
            if proposal_queries.len() > 0 {
                for (last_eth_block_height, round)  in proposal_queries.into_iter(){
                    let proposal_set: Vec<types::Proposal::<T::AccountId>> =
                        <ProposalMap<T>>::get(last_eth_block_height).into_iter().filter(|p| p.round == round).collect();

                    // No dispute on this proposal, confirm all blocks
                    if proposal_set.len() == 1 {
                        for (idx, &h) in proposal_set[0].headers.iter().enumerate() {
                            let mut previous_proposal_set: Vec<types::Proposal::<T::AccountId>> = Vec::new();
                            <ProposalMap<T>>::mutate(h.block_height, |v|{
                                let mut remind_proposal_set: Vec<types::Proposal::<T::AccountId>> = Vec::new();
                                for p in v.iter(){
                                    if p.round == idx as u32 + 1 {
                                        previous_proposal_set.push(p.clone());
                                    } else {
                                        remind_proposal_set.push(p.clone());
                                    }
                                }
                                remind_proposal_set
                            });
                        }

                        let mut proposal_extend_from = proposal_set[0].extend_from;
                        let mut round = proposal_set[0].round;
                        loop {
                            round -= 1;
                            if proposal_extend_from.is_none(){ // The root proposal
                                Self::reward_by_proposal(&proposal_set[0], BOND_VALUE);
                                break
                            } else {
                                let previous_proposal_id = proposal_extend_from.unwrap();
                                let mut previous_proposal_set: Vec<types::Proposal::<T::AccountId>> = Vec::new();
                                <ProposalMap<T>>::mutate(previous_proposal_id.0, |v|{
                                    let mut remind_proposal_set: Vec<types::Proposal::<T::AccountId>> = Vec::new();
                                    for p in v.iter(){
                                        if p.round == round {
                                            previous_proposal_set.push(p.clone());
                                        } else {
                                            remind_proposal_set.push(p.clone());
                                        }
                                    }
                                    remind_proposal_set
                                });
                                let mut honest_one: Option<types::Proposal::<T::AccountId>> = None;
                                let number_of_lier = previous_proposal_set.len() as u32 - 1;
                                for p in previous_proposal_set.into_iter() {
                                    // In production compare hash here
                                    if p.headers.last().unwrap().lie ==  previous_proposal_id.1 {
                                        honest_one = Some(p);
                                    } else {
                                        Self::slash_by_proposal(&p, BOND_VALUE);
                                    }
                                }
                                if let Some(p) = honest_one {
                                    Self::reward_by_proposal(&p, BOND_VALUE * number_of_lier);
                                    proposal_extend_from = p.extend_from;
                                } else {
                                    // the last propsoal should be extend from one of the proposal
                                    // and we deem it as honest
                                    panic!("There should be a one");
                                }
                            }
                        }
                    } else {
                        // There are still more than one voice, add samples and open the next round
                        let mut current_samples = Samples::get(proposal_set[0].headers[0].block_height);
                        let last_comfirm_block_height = match LastConfirmedHeader::get() {
                            Some(h) => h.block_height,
                            None => 0
                        };
                        let new_samples = Self::update_samples(&mut current_samples, last_comfirm_block_height);
                        Self::set_samples(&new_samples);
                    }
                }
            }
        }
    }
}
impl<T: Trait> Module<T> {
    fn get_current_round_from_submit_length(length: usize) -> u32 {
        if length == 1 {
            return 1;
        } else {
            return num_bits::<isize>() as u32 - (length - 1).leading_zeros() + 1;
        }
    }
    fn set_samples(new_samples: &Vec<types::EthereumBlockHeightType>) {
        if new_samples.len() > 1 {
            let samples = Samples::get(new_samples[0]);
            if samples.len() == 0 {
                panic!("setup samples should be extend from before");
            }
            for (idx, s) in samples.into_iter().enumerate() {
                if s != new_samples[idx] {
                    panic!("setup samples should be extend from before");
                }
            }
        }
        Samples::insert(new_samples[0], new_samples);
    }
    fn update_samples(
        _current_samples: &mut Vec<types::EthereumBlockHeightType>,
        _last_comfirm_block_height: types::EthereumBlockHeightType,
    ) -> Vec<types::EthereumBlockHeightType> {
        // Vec::new()
        vec![1000, 500]
    }
    fn reward_by_proposal(proposal: &types::Proposal<T::AccountId>, value: u32) {
        #[cfg(feature = "std")]
        println!("reward {} to {:?}", value, &proposal.relayer);
    }
    fn slash_by_proposal(proposal: &types::Proposal<T::AccountId>, value: u32) {
        #[cfg(feature = "std")]
        println!("slash {} to {:?}", value, &proposal.relayer);
    }
}
