use crate::mock::*;
use frame_support::assert_ok;

#[test]
fn test_handle_confirm_blocks_affinity() {
    new_test_ext().execute_with(|| {
        assert_ok!(SampleModule::confirm(Origin::signed(1), 42));
        assert_eq!(SampleModule::handle_confirm_blocks_affinity(0, 100, 50), 43);
        assert_eq!(SampleModule::handle_confirm_blocks_affinity(0, 100, 35), 41);
    });
}
