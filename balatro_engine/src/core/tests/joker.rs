use crate::core::card::Card;
use crate::core::enums::{ALL_RANKS, Edition, Enhancement, Seal, Suit};
use crate::core::game::GameState;
use crate::core::joker::{Joker, JokerData, JokerStructure, Jokers, TriggerEvent};
use crate::core::joker_types::{JokerKind, JokerUpdate};

#[test]
fn mail_in_rebate_triggers_for_matching_discarded_cards() {
    let mut jokers = Jokers::new();
    jokers.add(Joker::create_joker(JokerKind::MailInRebate));
    let mut game_state = GameState::new(7);
    jokers.initialize(&mut game_state);

    let target = match &jokers.jokers[0].structure {
        JokerStructure::Normal(JokerData::Econ(data)) => data.rank.as_ref().unwrap()[0],
        _ => panic!("Mail-In Rebate did not contain economic data"),
    };
    let non_matching_rank = ALL_RANKS
        .iter()
        .copied()
        .find(|rank| *rank != target)
        .unwrap();

    let discarded_cards = vec![
        Card::new(
            target,
            Suit::Hearts,
            Enhancement::None,
            Edition::None,
            Seal::None,
        ),
        Card::new(
            target,
            Suit::Diamonds,
            Enhancement::None,
            Edition::None,
            Seal::None,
        ),
        Card::new(
            target,
            Suit::Clubs,
            Enhancement::None,
            Edition::None,
            Seal::None,
        ),
        Card::new(
            non_matching_rank,
            Suit::Spades,
            Enhancement::None,
            Edition::None,
            Seal::None,
        ),
        Card::new(
            non_matching_rank,
            Suit::Spades,
            Enhancement::None,
            Edition::None,
            Seal::None,
        ),
    ];

    jokers.trigger(
        TriggerEvent::Discard {
            cards: &discarded_cards,
        },
        &mut game_state,
    );

    assert_eq!(game_state.money, 15);
}

#[test]
fn idol_is_initialized_and_updates_with_the_game_rng() {
    let mut jokers = Jokers::new();
    jokers.add(Joker::create_joker(JokerKind::Idol));
    let mut game_state = GameState::new(11);

    jokers.initialize(&mut game_state);

    match &jokers.jokers[0].structure {
        JokerStructure::Normal(JokerData::Scoring(data)) => {
            assert!(data.rank.is_some());
            assert!(data.suit.is_some());
        }
        _ => panic!("Idol did not contain scoring data"),
    }
}

#[test]
fn every_configured_persistent_update_is_registered() {
    let expected = [
        (JokerKind::Glass, &[JokerUpdate::CardDestroyed][..]),
        (
            JokerKind::CeremonialDagger,
            &[JokerUpdate::BlindSelected][..],
        ),
        (
            JokerKind::HitTheRoad,
            &[JokerUpdate::CardsDiscarded, JokerUpdate::RoundStarted][..],
        ),
        (JokerKind::RideTheBus, &[JokerUpdate::HandCompleted][..]),
        (JokerKind::Invisible, &[JokerUpdate::RoundCompleted][..]),
        (JokerKind::Canio, &[JokerUpdate::CardDestroyed][..]),
        (JokerKind::Yorick, &[JokerUpdate::CardsDiscarded][..]),
        (JokerKind::Runner, &[JokerUpdate::HandCompleted][..]),
        (JokerKind::IceCream, &[JokerUpdate::HandCompleted][..]),
        (JokerKind::Constellation, &[JokerUpdate::PlanetCardUsed][..]),
        (
            JokerKind::Green,
            &[
                JokerUpdate::HandCompleted,
                JokerUpdate::DiscardActionCompleted,
            ][..],
        ),
        (JokerKind::RedCard, &[JokerUpdate::BoosterPackSkipped][..]),
        (JokerKind::Madness, &[JokerUpdate::BlindSelected][..]),
        (JokerKind::SquareJoker, &[JokerUpdate::HandCompleted][..]),
        (JokerKind::Hologram, &[JokerUpdate::DeckChanged][..]),
        (JokerKind::Obelisk, &[JokerUpdate::HandCompleted][..]),
        (JokerKind::TurtleBean, &[JokerUpdate::RoundCompleted][..]),
        (JokerKind::LuckyCat, &[JokerUpdate::LuckyCardSucceeded][..]),
        (JokerKind::FlashCard, &[JokerUpdate::ShopRerolled][..]),
        (JokerKind::Popcorn, &[JokerUpdate::RoundCompleted][..]),
        (JokerKind::Ramen, &[JokerUpdate::CardsDiscarded][..]),
        (JokerKind::Seltzer, &[JokerUpdate::HandCompleted][..]),
        (JokerKind::SpareTrousers, &[JokerUpdate::HandCompleted][..]),
        (
            JokerKind::Campfire,
            &[JokerUpdate::AfterCardSold, JokerUpdate::BossBlindCompleted][..],
        ),
        (JokerKind::Castle, &[JokerUpdate::CardsDiscarded][..]),
    ];

    for (kind, updates) in expected {
        let joker = Joker::create_joker(kind);
        let expected_mask = updates.iter().fold(0, |mask, update| mask | update.mask());
        assert_eq!(joker.update_mask & expected_mask, expected_mask, "{kind:?}");
    }
}

#[test]
fn joker_categories_track_ordered_indices_for_each_behavior() {
    let mut jokers = Jokers::new();
    for kind in [
        JokerKind::Greedy,
        JokerKind::Mime,
        JokerKind::Hack,
        JokerKind::RiffRaff,
        JokerKind::Blueprint,
        JokerKind::Dusk,
        JokerKind::FourFingers,
        JokerKind::Glass,
    ] {
        jokers.add(Joker::create_joker(kind));
    }

    assert_eq!(jokers.on_played, vec![0, 2, 5]);
    assert_eq!(jokers.on_held, vec![1]);
    assert_eq!(jokers.played_retriggers, vec![2, 5]);
    assert_eq!(
        jokers
            .on_played_jokers()
            .map(Joker::kind)
            .collect::<Vec<_>>(),
        vec![JokerKind::Greedy, JokerKind::Hack, JokerKind::Dusk]
    );
}
