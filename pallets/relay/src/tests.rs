use crate::{mock::*, Error};
use frame_support::{assert_noop, assert_ok};

#[test]
fn test_set_last_comfirm_header() {
    new_test_ext().execute_with(|| {
        assert_ok!(Relay::set_last_comfirm_header(
            Origin::signed(1),
            crate::types::EthHeader {
                lie: 0,
                block_height: 1
            }
        ));
        let relay_header = Relay::last_comfirm_header();
        assert!(relay_header.is_some());
        let header = relay_header.unwrap().header;
        assert_eq!(
            header,
            crate::types::EthHeader {
                lie: 0,
                block_height: 1
            }
        );
    });
}

#[test]
fn test_rehect_set_lie_last_comfirm_header() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Relay::set_last_comfirm_header(
                Origin::signed(1),
                crate::types::EthHeader {
                    lie: 1,
                    block_height: 1
                }
            ),
            Error::<Test>::HeaderInvalid
        );
        let relay_header = Relay::last_comfirm_header();
        assert!(relay_header.is_none());
    });
}
