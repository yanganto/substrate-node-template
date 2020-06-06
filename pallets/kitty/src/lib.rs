#![cfg_attr(not(feature = "std"), no_std)]

use balances;
use codec::{Decode, Encode};
use frame_support::{decl_error, decl_event, decl_module, decl_storage, dispatch};
use frame_system::{self as system, ensure_signed};

// NOTE not really std, but similar
use sp_std::{cmp, prelude::Vec};

use crate::sp_api_hidden_includes_decl_storage::hidden_include::traits::Randomness;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[derive(Encode, Decode, Default, Clone, PartialEq)]
pub struct Kitty<Hash, Balance> {
    id: Hash,
    dna: Hash,
    price: Balance,
    gen: u64,
}

pub trait Trait: system::Trait + balances::Trait {
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

decl_storage! {
    trait Store for Module<T: Trait> as Kitty {
        OwnedKitty get(fn kitty_of_owner) : map hasher(identity) T::AccountId => Vec<T::Hash>;
        KittyOwner get(fn owner_of) : map hasher(blake2_128_concat) T::Hash => Option<T::AccountId>;
        Kitties get(fn kitties): map hasher(blake2_128_concat) T::Hash => Kitty<T::Hash, T::Balance>;
    }
}

decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as system::Trait>::AccountId,
        Hash = <T as system::Trait>::Hash,
    {
        Created(AccountId, Hash),
    }
);

decl_error! {
    pub enum Error for Module<T: Trait> {
        // the Kiity is unique
        CreatedKittyExist,
        KittyNotExist,
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        type Error = Error<T>;
        fn deposit_event() = default;

        #[weight = 0]
        pub fn create_kitty(origin) -> dispatch::DispatchResult {

            let owner = ensure_signed(origin)?;

            // add ramdomness_collective_flip pallet to create random hash
            let r = <pallet_randomness_collective_flip::Module<T>>::random_seed();

            if <KittyOwner<T>>::contains_key(r) {
               return Err(<Error<T>>::CreatedKittyExist)?;
            }


            let new_kitty = Kitty {
                id: r,
                dna: r,
                price: 0.into(),
                gen: 0,
            };

            <Kitties<T>>::insert(r, new_kitty);
            <KittyOwner<T>>::insert(r, &owner);
            <OwnedKitty<T>>::mutate(&owner, |v| v.push(r));

            Ok(())
        }

        #[weight = 0]
        fn breed_kitty(origin, kitty_id_1: T::Hash, kitty_id_2: T::Hash) -> dispatch::DispatchResult {
            let ownner = ensure_signed(origin)?;

            if <KittyOwner<T>>::get(kitty_id_1) != Some(ownner.clone()) || <KittyOwner<T>>::get(kitty_id_2) != Some(ownner.clone()) {
               return Err(<Error<T>>::KittyNotExist)?;
            }

            let kitty_1 = <Kitties<T>>::get(kitty_id_1);
            let kitty_2 = <Kitties<T>>::get(kitty_id_2);

            let r = <pallet_randomness_collective_flip::Module<T>>::random_seed();

            let mut final_dna = kitty_1.dna;

            for (i, (dna_2_element, r)) in kitty_2.dna.as_ref().iter().zip(r.as_ref().iter()).enumerate() {

                // half possibility to mix kitty 2 dna into final_dna
                if r % 2 == 0 {
                    final_dna.as_mut()[i] = *dna_2_element;
                }
            }

            let new_kitty = Kitty {
                id: r,
                dna: final_dna,
                price: 0.into(),
                gen: cmp::max(kitty_1.gen, kitty_2.gen) + 1,
            };

            <Kitties<T>>::insert(r, new_kitty);
            <KittyOwner<T>>::insert(r, &ownner);
            <OwnedKitty<T>>::mutate(&ownner, |v| v.push(r));

            Ok(())
        }
    }
}
