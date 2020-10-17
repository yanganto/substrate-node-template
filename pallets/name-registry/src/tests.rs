use crate::{mock::*, Error};
use frame_support::{assert_noop, assert_ok};

#[test]
fn register_a_name() {
    new_test_ext().execute_with(|| {
        assert_eq!(
            Balances::total_issuance(),
            (u32::max_value() as u64) + (u16::max_value() as u64) + (u8::max_value() as u64)
        );
        let name = vec![1, 2, 3, 4];
        Timestamp::set_timestamp(42);
        assert_eq!(Balances::free_balance(&1), 255);
        assert_ok!(NameRegisteredModule::register_name(
            Origin::signed(1),
            name.clone()
        ));
        let maybe_a_name = NameRegisteredModule::names(name.clone());
        assert_eq!(maybe_a_name.is_some(), true);
        assert_eq!(maybe_a_name.unwrap().expired, 604800042);
        assert_eq!(Balances::free_balance(&1), 255 - (name.len() as u64));
    });
}

#[test]
fn correct_error_for_register_twice() {
    new_test_ext().execute_with(|| {
        let name = vec![1, 2, 3, 4];
        assert_eq!(Balances::free_balance(&1), 255);
        assert_eq!(Balances::free_balance(&2), 65535);
        assert_ok!(NameRegisteredModule::register_name(
            Origin::signed(1),
            name.clone()
        ));
        assert_noop!(
            NameRegisteredModule::register_name(Origin::signed(2), name.clone()),
            Error::<Test>::NameAlreadyRegistered
        );
        assert_eq!(Balances::free_balance(&1), 255 - (name.len() as u64));
        assert_eq!(Balances::free_balance(&2), 65535);
    });
}

#[test]
fn correct_error_for_register_invalid_name() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            NameRegisteredModule::register_name(Origin::signed(2), vec![]),
            Error::<Test>::NameInvalid
        );
    });
}

#[test]
fn renew_a_registration() {
    new_test_ext().execute_with(|| {
        assert_ok!(NameRegisteredModule::register_name(
            Origin::signed(1),
            vec![1, 2, 3, 4]
        ));
        assert_ok!(NameRegisteredModule::renew_register_name(
            Origin::signed(1),
            vec![1, 2, 3, 4]
        ));
    });
}

#[test]
fn correct_error_for_renew_registration_by_others() {
    new_test_ext().execute_with(|| {
        assert_ok!(NameRegisteredModule::register_name(
            Origin::signed(1),
            vec![1, 2, 3, 4]
        ));
        assert_noop!(
            NameRegisteredModule::renew_register_name(Origin::signed(2), vec![1, 2, 3, 4]),
            Error::<Test>::NameRegisteredByOther
        );
    });
}

#[test]
fn correct_error_for_renew_a_nonexist_registration() {
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(42);
        assert_noop!(
            NameRegisteredModule::renew_register_name(Origin::signed(1), vec![1, 2, 3, 4]),
            Error::<Test>::NameNotExist
        );
    });
}

#[test]
fn correct_error_for_unregister_a_name() {
    new_test_ext().execute_with(|| {
        let name = vec![1, 2, 3, 4];
        assert_ok!(NameRegisteredModule::register_name(
            Origin::signed(1),
            name.clone()
        ));
        assert_noop!(
            NameRegisteredModule::ungister_name(Origin::signed(1), name.clone()),
            Error::<Test>::RegisteredTimeNotOver
        );
    });
}
