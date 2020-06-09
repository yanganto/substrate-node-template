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
fn test_submission_take_over_should_follow_sample() {
    new_test_ext().execute_with(|| {
        assert_ok!(Relay::submit(
            Origin::signed(1),
            vec![crate::types::EthHeader {
                lie: 1,
                block_height: 1000
            }]
        ));

        // Do challenge with header
        assert_ok!(Relay::submit(
            Origin::signed(2),
            vec![crate::types::EthHeader {
                lie: 0,
                block_height: 1000
            }]
        ));

        assert_noop!(
            Relay::submit(
                Origin::signed(2),
                vec![
                    crate::types::EthHeader {
                        lie: 0,
                        block_height: 1000
                    },
                    crate::types::EthHeader {
                        lie: 0,
                        block_height: 500
                    }
                ]
            ),
            Error::<Test>::NotComplyWithSamples
        );

        // simulate over challenge time and the sample set extends
        Relay::set_samples(&vec![1000, 500]);

        assert_ok!(Relay::submit(
            Origin::signed(2),
            vec![
                crate::types::EthHeader {
                    lie: 0,
                    block_height: 1000
                },
                crate::types::EthHeader {
                    lie: 0,
                    block_height: 500
                }
            ]
        ));
    });
}

#[test]
fn test_current_submit_round_calculation() {
    new_test_ext().execute_with(|| {
        assert!(Relay::get_current_round_from_submit_length(1) == 1);
        assert!(Relay::get_current_round_from_submit_length(2) == 2u32);
        assert!(Relay::get_current_round_from_submit_length(4) == 3u32);
        assert!(Relay::get_current_round_from_submit_length(8) == 4u32);
        assert!(Relay::get_current_round_from_submit_length(16) == 5u32);
    });
}
