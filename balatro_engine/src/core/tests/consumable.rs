use super::*;

use crate::core::card::Card;
use crate::core::consumable::{
    ConsumableTarget, ConsumableUseError, OwnedConsumable, edition_for_roll,
    joker_edition_for_roll, judgement_edition, judgement_rarity,
};
use crate::core::enums::{
    Consumable, Edition, Enhancement, Planet, PokerHand, Rank, Seal, Spectral, Suit, Tarot,
};
use crate::core::hand::Hand;
use crate::core::joker::Joker;
use crate::core::joker_types::{JokerEdition, JokerKind, JokerRarity};

fn hand_with_card(rank: Rank) -> Hand {
    let mut hand = Hand::new();
    hand.add_card(Card::new(
        rank,
        Suit::Spades,
        Enhancement::None,
        Edition::None,
        Seal::None,
    ));
    hand
}

#[test]
fn tarot_card_mutates_target_and_leaves_inventory_after_use() {
    let mut state = GameState::new(1);
    state
        .add_consumable(Consumable::Tarot(Tarot::Magician))
        .unwrap();
    let mut hand = hand_with_card(Rank::Seven);

    state
        .use_consumable(0, &mut hand, ConsumableTarget::hand(vec![0]))
        .unwrap();

    assert_eq!(hand.card(0).unwrap().enhancement(), Enhancement::Lucky);
    assert!(state.consumables.is_empty());
    assert_eq!(state.counters.tarot_cards_used, 1);
    assert_eq!(
        state.last_consumable,
        Some(OwnedConsumable::new(Consumable::Tarot(Tarot::Magician), 3,))
    );
}

#[test]
fn invalid_target_does_not_consume_or_mutate_card() {
    let mut state = GameState::new(2);
    state
        .add_consumable(Consumable::Tarot(Tarot::Lovers))
        .unwrap();
    let mut hand = hand_with_card(Rank::Seven);
    let original_id = hand.card(0).unwrap().id();

    let result = state.use_consumable(0, &mut hand, ConsumableTarget::hand(vec![1]));

    assert_eq!(result, Err(ConsumableUseError::InvalidTarget));
    assert_eq!(hand.card(0).unwrap().id(), original_id);
    assert_eq!(state.consumables.len(), 1);
}

#[test]
fn planet_cards_update_the_explicit_hand_level_table() {
    let mut state = GameState::new(3);
    state
        .add_consumable(Consumable::Planet(Planet::Saturn))
        .unwrap();
    let mut hand = Hand::new();

    state
        .use_consumable(0, &mut hand, ConsumableTarget::default())
        .unwrap();

    let level = state.hand_level(PokerHand::Straight);
    assert_eq!(level.level, 2);
    assert_eq!(level.chips, 30);
    assert_eq!(level.mult, 3);
}

#[test]
fn black_hole_upgrades_every_poker_hand() {
    let mut state = GameState::new(4);
    state
        .add_consumable(Consumable::Spectral(
            crate::core::enums::Spectral::BlackHole,
        ))
        .unwrap();
    let mut hand = Hand::new();

    state
        .use_consumable(0, &mut hand, ConsumableTarget::default())
        .unwrap();

    assert_eq!(state.hand_level(PokerHand::HighCard).level, 2);
    assert_eq!(state.hand_level(PokerHand::FlushFive).level, 2);
}

#[test]
fn tarot_creation_and_fool_use_the_previous_tarot() {
    let mut state = GameState::new(5);
    state
        .add_consumable(Consumable::Tarot(Tarot::HighPriestess))
        .unwrap();
    let mut hand = Hand::new();

    state
        .use_consumable(0, &mut hand, ConsumableTarget::default())
        .unwrap();
    assert_eq!(state.consumables.len(), 2);
    assert_eq!(
        state.last_consumable,
        Some(OwnedConsumable::new(
            Consumable::Tarot(Tarot::HighPriestess),
            3,
        ))
    );

    state.consumables.clear();
    state
        .add_consumable(Consumable::Tarot(Tarot::Fool))
        .unwrap();
    state
        .use_consumable(0, &mut hand, ConsumableTarget::default())
        .unwrap();

    assert_eq!(
        state.consumables,
        vec![OwnedConsumable::new(
            Consumable::Tarot(Tarot::HighPriestess),
            3,
        )]
    );
    assert_eq!(
        state.last_consumable,
        Some(OwnedConsumable::new(
            Consumable::Tarot(Tarot::HighPriestess),
            3,
        ))
    );
}

#[test]
fn death_copies_all_card_properties_into_the_left_target() {
    let mut state = GameState::new(6);
    state
        .add_consumable(Consumable::Tarot(Tarot::Death))
        .unwrap();
    let mut hand = Hand::new();
    hand.add_card(Card::new(
        Rank::Two,
        Suit::Hearts,
        Enhancement::Lucky,
        Edition::Foil,
        Seal::Blue,
    ));
    hand.add_card(Card::new(
        Rank::Ace,
        Suit::Clubs,
        Enhancement::Steel,
        Edition::Polychrome,
        Seal::Gold,
    ));

    state
        .use_consumable(0, &mut hand, ConsumableTarget::hand(vec![0, 1]))
        .unwrap();

    let left = hand.card(0).unwrap();
    let right = hand.card(1).unwrap();
    assert_eq!(left.rank(), right.rank());
    assert_eq!(left.suit(), right.suit());
    assert_eq!(left.enhancement(), right.enhancement());
    assert_eq!(left.edition(), right.edition());
    assert_eq!(left.seal(), right.seal());
}

#[test]
fn spectral_card_mutations_apply_seals_editions_and_hand_size_changes() {
    let mut state = GameState::new(7);
    state
        .add_consumable(Consumable::Spectral(crate::core::enums::Spectral::DejaVu))
        .unwrap();
    let mut hand = hand_with_card(Rank::Seven);
    state
        .use_consumable(0, &mut hand, ConsumableTarget::hand(vec![0]))
        .unwrap();
    assert_eq!(hand.card(0).unwrap().seal(), Seal::Red);

    state
        .add_consumable(Consumable::Spectral(crate::core::enums::Spectral::Aura))
        .unwrap();
    state
        .use_consumable(0, &mut hand, ConsumableTarget::hand(vec![0]))
        .unwrap();
    assert!(matches!(
        hand.card(0).unwrap().edition(),
        Edition::Foil | Edition::Holographic | Edition::Polychrome
    ));

    state
        .add_consumable(Consumable::Spectral(crate::core::enums::Spectral::Ouija))
        .unwrap();
    state
        .use_consumable(0, &mut hand, ConsumableTarget::default())
        .unwrap();
    assert_eq!(state.hand_size(), 7);
    assert!(matches!(
        hand.card(0).unwrap().rank(),
        Rank::Two
            | Rank::Three
            | Rank::Four
            | Rank::Five
            | Rank::Six
            | Rank::Seven
            | Rank::Eight
            | Rank::Nine
            | Rank::Ten
            | Rank::Jack
            | Rank::Queen
            | Rank::King
            | Rank::Ace
    ));
}

#[test]
fn spectral_joker_effects_preserve_the_selected_joker_rules() {
    let mut state = GameState::new(8);
    state.jokers.add(Joker::create_joker(JokerKind::Joker));
    state.jokers.add(Joker::create_joker(JokerKind::Greedy));
    state
        .add_consumable(Consumable::Spectral(crate::core::enums::Spectral::Hex))
        .unwrap();
    let mut hand = Hand::new();

    state
        .use_consumable(0, &mut hand, ConsumableTarget::default())
        .unwrap();

    assert_eq!(state.jokers.as_slice().len(), 1);
    assert_eq!(
        state.jokers.as_slice()[0].edition(),
        JokerEdition::Polychrome
    );

    state
        .jokers
        .get_mut(0)
        .unwrap()
        .set_edition(JokerEdition::Negative);
    state
        .add_consumable(Consumable::Spectral(crate::core::enums::Spectral::Ankh))
        .unwrap();
    state
        .use_consumable(0, &mut hand, ConsumableTarget::default())
        .unwrap();
    assert_eq!(state.jokers.as_slice()[0].edition(), JokerEdition::None);
}

#[test]
fn spectral_generation_requires_the_hand_or_joker_target_to_exist() {
    let mut state = GameState::new(9);
    state
        .add_consumable(Consumable::Spectral(crate::core::enums::Spectral::Familiar))
        .unwrap();
    let mut hand = Hand::new();

    let result = state.use_consumable(0, &mut hand, ConsumableTarget::default());

    assert_eq!(result, Err(ConsumableUseError::NoAvailableTarget));
    assert_eq!(state.consumables.len(), 1);
    assert!(state.jokers.as_slice().is_empty());
}

#[test]
fn rare_and_legendary_creation_use_the_existing_joker_factory() {
    let mut state = GameState::new(10);
    state.money = 25;
    state
        .add_consumable(Consumable::Spectral(crate::core::enums::Spectral::Wraith))
        .unwrap();
    let mut hand = Hand::new();
    state
        .use_consumable(0, &mut hand, ConsumableTarget::default())
        .unwrap();
    assert_eq!(state.money, 0);
    assert_eq!(state.jokers.as_slice().len(), 1);
    assert_eq!(state.jokers.as_slice()[0].rarity(), JokerRarity::Rare);

    state
        .add_consumable(Consumable::Spectral(crate::core::enums::Spectral::Soul))
        .unwrap();
    state
        .use_consumable(0, &mut hand, ConsumableTarget::default())
        .unwrap();
    assert_eq!(state.jokers.as_slice().len(), 2);
    assert!(
        state
            .jokers
            .as_slice()
            .iter()
            .any(|joker| joker.rarity() == JokerRarity::Legendary)
    );
}

#[test]
fn every_tarot_branch_accepts_a_valid_use() {
    let cards = [
        Tarot::Fool,
        Tarot::Magician,
        Tarot::HighPriestess,
        Tarot::Emperor,
        Tarot::Empress,
        Tarot::Hierophant,
        Tarot::Lovers,
        Tarot::Chariot,
        Tarot::Justice,
        Tarot::Hermit,
        Tarot::WheelOfFortune,
        Tarot::Strength,
        Tarot::HangedMan,
        Tarot::Death,
        Tarot::Temperance,
        Tarot::Devil,
        Tarot::Tower,
        Tarot::Star,
        Tarot::Moon,
        Tarot::Sun,
        Tarot::Judgement,
        Tarot::World,
    ];

    for (offset, card) in cards.into_iter().enumerate() {
        let mut state = GameState::new(100 + offset as u64);
        let mut hand = hand_with_card(Rank::Seven);
        hand.add_card(Card::new(
            Rank::Eight,
            Suit::Hearts,
            Enhancement::None,
            Edition::None,
            Seal::None,
        ));
        state.last_consumable = Some(OwnedConsumable::new(Consumable::Tarot(Tarot::Magician), 3));
        if matches!(
            card,
            Tarot::WheelOfFortune | Tarot::Temperance | Tarot::Judgement
        ) {
            state.jokers.add(Joker::create_joker(JokerKind::Joker));
        }
        state.add_consumable(Consumable::Tarot(card)).unwrap();

        let target = match card {
            Tarot::Magician
            | Tarot::Empress
            | Tarot::Hierophant
            | Tarot::Strength
            | Tarot::HangedMan
            | Tarot::Star
            | Tarot::Moon
            | Tarot::Sun
            | Tarot::World
            | Tarot::Death => ConsumableTarget::hand(vec![0, 1]),
            Tarot::Lovers | Tarot::Chariot | Tarot::Justice | Tarot::Devil | Tarot::Tower => {
                ConsumableTarget::hand(vec![0])
            }
            _ => ConsumableTarget::default(),
        };

        assert!(
            state.use_consumable(0, &mut hand, target).is_ok(),
            "{card:?}"
        );
    }
}

#[test]
fn every_spectral_branch_accepts_a_valid_use() {
    let cards = [
        Spectral::Familiar,
        Spectral::Grim,
        Spectral::Incantation,
        Spectral::Talisman,
        Spectral::Aura,
        Spectral::Wraith,
        Spectral::Sigil,
        Spectral::Ouija,
        Spectral::Ectoplasm,
        Spectral::Immolate,
        Spectral::Ankh,
        Spectral::DejaVu,
        Spectral::Hex,
        Spectral::Trance,
        Spectral::Medium,
        Spectral::Cryptid,
        Spectral::Soul,
        Spectral::BlackHole,
    ];

    for (offset, card) in cards.into_iter().enumerate() {
        let mut state = GameState::new(200 + offset as u64);
        let mut hand = hand_with_card(Rank::Seven);
        for rank in [
            Rank::Eight,
            Rank::Nine,
            Rank::Ten,
            Rank::Jack,
            Rank::Queen,
            Rank::King,
        ] {
            hand.add_card(Card::new(
                rank,
                Suit::Hearts,
                Enhancement::None,
                Edition::None,
                Seal::None,
            ));
        }
        if matches!(
            card,
            Spectral::Wraith
                | Spectral::Ectoplasm
                | Spectral::Ankh
                | Spectral::Hex
                | Spectral::Soul
        ) {
            state.jokers.add(Joker::create_joker(JokerKind::Joker));
            state.jokers.add(Joker::create_joker(JokerKind::Greedy));
        }
        state.add_consumable(Consumable::Spectral(card)).unwrap();

        let target = match card {
            Spectral::Talisman
            | Spectral::Aura
            | Spectral::DejaVu
            | Spectral::Trance
            | Spectral::Medium
            | Spectral::Cryptid => ConsumableTarget::hand(vec![0]),
            _ => ConsumableTarget::default(),
        };

        assert!(
            state.use_consumable(0, &mut hand, target).is_ok(),
            "{card:?}"
        );
    }
}

#[test]
fn judgement_uses_the_updated_rarity_and_edition_boundaries() {
    assert_eq!(judgement_rarity(0), JokerRarity::Common);
    assert_eq!(judgement_rarity(6_999), JokerRarity::Common);
    assert_eq!(judgement_rarity(7_000), JokerRarity::Uncommon);
    assert_eq!(judgement_rarity(9_499), JokerRarity::Uncommon);
    assert_eq!(judgement_rarity(9_500), JokerRarity::Rare);

    assert_eq!(judgement_edition(0), JokerEdition::Negative);
    assert_eq!(judgement_edition(29), JokerEdition::Negative);
    assert_eq!(judgement_edition(30), JokerEdition::Polychrome);
    assert_eq!(judgement_edition(59), JokerEdition::Polychrome);
    assert_eq!(judgement_edition(60), JokerEdition::Holographic);
    assert_eq!(judgement_edition(199), JokerEdition::Holographic);
    assert_eq!(judgement_edition(200), JokerEdition::Foil);
    assert_eq!(judgement_edition(399), JokerEdition::Foil);
    assert_eq!(judgement_edition(400), JokerEdition::None);
    assert_eq!(judgement_edition(9_999), JokerEdition::None);
}

#[test]
fn aura_and_wheel_share_the_requested_edition_distribution() {
    assert_eq!(edition_for_roll(0), Edition::Foil);
    assert_eq!(edition_for_roll(49), Edition::Foil);
    assert_eq!(edition_for_roll(50), Edition::Holographic);
    assert_eq!(edition_for_roll(84), Edition::Holographic);
    assert_eq!(edition_for_roll(85), Edition::Polychrome);
    assert_eq!(edition_for_roll(99), Edition::Polychrome);

    assert_eq!(joker_edition_for_roll(0), JokerEdition::Foil);
    assert_eq!(joker_edition_for_roll(50), JokerEdition::Holographic);
    assert_eq!(joker_edition_for_roll(85), JokerEdition::Polychrome);
}
