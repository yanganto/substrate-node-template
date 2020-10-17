#![cfg_attr(not(feature = "std"), no_std)]
use codec::{Decode, Encode};
/// A vanity name registering system resistant against frontrunning.
///
/// An unregistered name can be registered for a certain amount of time by locking a certain balance of an account.
/// After the registration expires, the account loses ownership of the name and his balance is unlocked.
/// The registration can be renewed by making an on-chain call to keep the name registered and balance locked.
use frame_support::{
    decl_error, decl_event, decl_module, decl_storage, dispatch,
    traits::{Currency, Get, ReservableCurrency},
};
use frame_system::ensure_signed;
use pallet_timestamp as timestamp;

// NOTE:
// This hot coding value is an asumption,
// that the total bytes used for storage the registered name is 4294967295 bytes
// This should be a configureable value in realy system, or a value load from storage and can be
// changed by root key.
const TOTAL_BYTE_FOR_NAME: u32 = u32::max_value();

// NOTE:
// The reasonable defaults for the locking period.
// This hot coding value is an asumption,
// that the once a name registered, it should not be change in a week
const NAME_RENEW_TIMESTAMP: u32 = 604800000; // 7 * 24 * 60 * 60 * 1000

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

type BalanceOf<T> =
    <<T as Trait>::Currency as Currency<<T as frame_system::Trait>::AccountId>>::Balance;

#[derive(Encode, Decode, Default, Clone, PartialEq)]
pub struct Name<Moment> {
    /// the name registered
    name: Vec<u8>,
    /// expired time in block number,
    expired: Moment,
}

pub trait Trait: frame_system::Trait + timestamp::Trait {
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
    type Currency: ReservableCurrency<Self::AccountId>;
}

decl_storage! {
    trait Store for Module<T: Trait> as TemplateModule {
        /// The Total names
        Names get(fn names): map hasher(blake2_128_concat) Vec<u8> => Option<Name<<T as timestamp::Trait>::Moment>>;
        /// The Names from someone
        NameOwner get(fn name_of) : map hasher(blake2_128_concat) Vec<u8> => Option<T::AccountId>;
    }
}

decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as frame_system::Trait>::AccountId,
        Moment = <T as timestamp::Trait>::Moment,
    {
        /// The event for someone registry a name
        NameRegistered(AccountId, Vec<u8>),
        /// The event for someone unregistry a name
        NameUnregistered(AccountId, Vec<u8>),
        /// The event for someone extend the registration of a name
        NameRegistrationExtended(AccountId, Vec<u8>, Moment),
    }
);

decl_error! {
    pub enum Error for Module<T: Trait> {
        /// Error for regist a name already registered
        NameAlreadyRegistered,
        /// The name is not valid
        NameInvalid,
        /// Error to renew a registration not by himself
        NameRegisteredByOther,
        /// Error for unregistery a name still in the promise registry time
        RegisteredTimeNotOver,
        /// The registed name is not exist
        NameNotExist,
    }
}
impl<T: Trait> Module<T> {
    /// calculate the register fee
    /// The fee to register the name depends directly on the size of the name.
    fn check_and_calculate_register_fee(name: &Vec<u8>) -> Option<BalanceOf<T>> {
        // The length of name is rescricted to prevent overflow issue
        if 0 < name.len() && name.len() < u32::max_value() as usize {
            // NOTE:
            // The total inssurance and the storage resource of names is related.
            // The idea is that the value of token == storage space
            // If there are more space to store things, more token can be issued more.
            //
            // It is quit safe to do this, because `u128 > u32 * u32`,
            // and it just safe to run in test, because `u64 = u32 * u32`
            Some(
                T::Currency::total_issuance() / TOTAL_BYTE_FOR_NAME.into()
                    * (name.len() as u32).into(),
            )
        } else {
            None
        }
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        type Error = Error<T>;

        fn deposit_event() = default;

        /// User can registery a valid name for 7 days,
        /// the fee affect by the size of name
        #[weight = 10_000 + T::DbWeight::get().writes(1)]
        pub fn register_name(origin, name: Vec<u8>) -> dispatch::DispatchResult {
            let who = ensure_signed(origin)?;
            let now = <timestamp::Module<T>>::get();

            // Make sure the name is valid and the user has enough fee
            let fee = if let Some(fee) = Self::check_and_calculate_register_fee(&name) {
                T::Currency::reserve(&who, fee)?;
                fee
            } else {
                Err(Error::<T>::NameInvalid)?
            };

            // Handle the conflict of name registration if exist
            if let Some(name_instance) = <Names<T>>::get(name.clone()) {
                if name_instance.expired > now {
                    T::Currency::unreserve(&who, fee);
                    Err(Error::<T>::NameAlreadyRegistered)?
                } else {
                    // The registration expires, the ownner loses ownership of the name
                    // and his balance is unlocked.
                    if let Some(ownner) = <NameOwner<T>>::take(name.clone()) {
                            T::Currency::unreserve(&ownner, fee);
                    } else {
                        panic!("registered name should be owned and valid")
                    }
                }
            }

            // Insert the new resistration
            let name_instance = Name::<<T as timestamp::Trait>::Moment>{
                name: name.clone(),
                expired: now + NAME_RENEW_TIMESTAMP.into(),
            };
            <Names<T>>::insert(name.clone(), name_instance);
            <NameOwner<T>>::insert(name.clone(), &who);

            Self::deposit_event(RawEvent::NameRegistered(who, name));
            Ok(())
        }

        /// User can unregistery a name over the promise time to unreserve the currency
        #[weight = 10_000 + T::DbWeight::get().writes(1)]
        pub fn ungister_name(origin, name: Vec<u8>) -> dispatch::DispatchResult {
            let who = ensure_signed(origin)?;
            let now = <timestamp::Module<T>>::get();

            if let Some(name_instance) = <Names<T>>::get(name.clone()) {
                if name_instance.expired > now {
                    Err(Error::<T>::RegisteredTimeNotOver)?
                } else {
                    <Names<T>>::take(name.clone());
                    if let (Some(ownner), Some(fee)) =
                        (<NameOwner<T>>::take(name.clone()), Self::check_and_calculate_register_fee(&name)){
                        if ownner == who {
                            T::Currency::unreserve(&ownner, fee);
                        } else {
                            Err(Error::<T>::NameRegisteredByOther)?
                        }
                    } else {
                        panic!("registered name should be owned and valid")
                    }
                }
            } else {
                Err(Error::<T>::NameNotExist)?
            }

            Self::deposit_event(RawEvent::NameUnregistered(who, name));
            Ok(())
        }
        /// The registration can be renewed by making an on-chain call to keep the name registered and balance locked.
        /// This method is limited by the owner of a registered name to prevent misuse case
        #[weight = 10_000 + T::DbWeight::get().writes(1)]
        pub fn renew_register_name(origin, name: Vec<u8>) -> dispatch::DispatchResult {
            let who = ensure_signed(origin)?;
            let now = <timestamp::Module<T>>::get();
            let new_time = now + NAME_RENEW_TIMESTAMP.into();

            if <Names<T>>::get(name.clone()).is_some() {
                if let Some(ownner) = <NameOwner<T>>::get(name.clone()) {
                    if ownner == who {
                        <Names<T>>::mutate(name.clone(), |_|{
                             Name::<<T as timestamp::Trait>::Moment>{
                                name: name.clone(),
                                expired: new_time.clone(),
                            }
                        });
                    } else {
                        // Only the registration owner can renew to prevent misuse case
                        Err(Error::<T>::NameRegisteredByOther)?
                    }
                } else {
                    panic!("registered name should be owned and valid")
                }
            } else {
                Err(Error::<T>::NameNotExist)?
            }

            Self::deposit_event(RawEvent::NameRegistrationExtended(who, name, new_time));
            Ok(())
        }
    }
}
