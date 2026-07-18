//! Talking to the player: freeze and resume.

use cardcrusher::duel::Duel;
use cardcrusher::processor::{DuelStatus, MSG_SELECT_CARD};

/// A task that needs an answer freezes the duel (`Awaiting`) after asking its
/// question; once a response is supplied it resumes and finishes.
#[test]
fn a_question_freezes_the_duel_until_answered() {
    let mut duel = Duel::new();
    duel.select_card();

    // Asks, then freezes.
    assert_eq!(duel.process(), DuelStatus::Awaiting);
    assert_eq!(
        duel.messages(),
        [MSG_SELECT_CARD],
        "it asked the right question"
    );

    // Answer → thaws and finishes.
    duel.set_response(&[0]);
    assert_eq!(duel.process(), DuelStatus::End);
}
