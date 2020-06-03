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
        let is_header = Relay::last_comfirm_header();
        assert!(is_header.is_some());
        let header = is_header.unwrap();
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
        let is_header = Relay::last_comfirm_header();
        assert!(is_header.is_none());
    });
}

#[test]
fn test_submit_header() {
    new_test_ext().execute_with(|| {
        assert_ok!(Relay::submit(
            Origin::signed(1),
            crate::types::EthHeader {
                lie: 0,
                block_height: 100
            }
        ));

        assert!(Relay::last_comfirm_header().is_none());
        assert_eq!(Relay::submit_headers().len(), 1);
        assert_eq!(Relay::submit_headers()[0], 100);
        assert_eq!(Relay::submit_headers_map(100).len(), 1);

        assert_ok!(Relay::submit(
            Origin::signed(2),
            crate::types::EthHeader {
                lie: 0,
                block_height: 100
            }
        ));
        assert_noop!(
            Relay::submit(
                Origin::signed(2),
                crate::types::EthHeader {
                    lie: 0,
                    block_height: 99
                }
            ),
            Error::<Test>::SubmitHeaderNotInSamplingList
        );

        assert_eq!(Relay::submit_headers_map(100).len(), 1);
        assert_eq!(Relay::submit_headers_map(100)[0].relayers.len(), 2);
        assert_eq!(Relay::submit_headers_map(100)[0].relayers[0], 1);
        assert_eq!(Relay::submit_headers_map(100)[0].relayers[1], 2);
        assert_eq!(Relay::submit_headers().len(), 1);
        assert_eq!(Relay::submit_headers()[0], 100);
        assert_eq!(Relay::next_sampling_header(), Some(50));
        assert_ok!(Relay::submit(
            Origin::signed(1),
            crate::types::EthHeader {
                lie: 0,
                block_height: 50
            }
        ));
        assert_eq!(Relay::submit_headers().len(), 2);
        assert_eq!(Relay::submit_headers_map(50).len(), 1);
        assert_eq!(Relay::submit_headers_map(50)[0].relayers.len(), 1);
        assert_eq!(Relay::next_sampling_header(), Some(25));
    });
}
