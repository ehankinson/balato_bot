use super::*;
use crate::core::blind::BlindKind;
use crate::core::card::Card;
use crate::core::consumable::ConsumableTarget;
use crate::core::enums::{BossBlinds, Edition, Enhancement, Planet, Rank, Seal, Suit};
use crate::core::joker_types::{JokerEdition, JokerKind};
use crate::core::pack::{BoosterPackKind, BoosterPackSize, ShopPack};
use crate::core::shop::{ShopError, ShopItem, ShopOfferKind, random_base_item};

fn card(rank: Rank, suit: Suit) -> Card {
    Card::new(rank, suit, Enhancement::None, Edition::None, Seal::None)
}

fn state_with(kinds: &[JokerKind]) -> GameState {
    let mut state = GameState::new(301);
    state.begin_blind_as(BlindKind::Small);
    for &kind in kinds {
        state.jokers.add(Joker::create_joker(kind));
    }
    state
}

fn play(state: &mut GameState, cards: &[Card]) -> ScoreResult {
    let mut hand = Hand::new();
    for card in cards {
        hand.add_card(card.clone());
    }
    state
        .play_hand(&mut hand, (0..cards.len() as u8).collect())
        .unwrap()
}

#[test]
fn splash_scores_kickers_without_changing_the_poker_hand() {
    let cards = [
        card(Rank::Two, Suit::Clubs),
        card(Rank::Two, Suit::Hearts),
        card(Rank::King, Suit::Spades),
    ];
    for debuffed in [false, true] {
        let mut state = state_with(&[JokerKind::Splash]);
        state.jokers.get_mut(0).unwrap().set_debuffed(debuffed);
        let score = play(&mut state, &cards);
        assert_eq!(score.hand, PokerHand::Pair);
        assert_eq!(score.chips, if debuffed { 14 } else { 24 });
        assert_eq!(
            score.scoring_data.scoring_played_cards.len(),
            if debuffed { 2 } else { 3 }
        );
    }
}

#[test]
fn splash_changes_chads_first_scoring_card_and_midas_eligibility() {
    let cards = [card(Rank::Two, Suit::Clubs), card(Rank::King, Suit::Hearts)];
    let mut state = state_with(&[JokerKind::Splash, JokerKind::HangingChad]);
    assert_eq!(play(&mut state, &cards).chips, 21);
    state.sell_joker(0).unwrap();
    assert_eq!(play(&mut state, &cards).chips, 35);

    for splash in [false, true] {
        let mut state = state_with(&[JokerKind::MidasMask, JokerKind::GoldenTicket]);
        if splash {
            state.jokers.add(Joker::create_joker(JokerKind::Splash));
        }
        let before = state.money;
        play(
            &mut state,
            &[
                card(Rank::Jack, Suit::Clubs),
                card(Rank::King, Suit::Hearts),
            ],
        );
        assert_eq!(state.money - before, if splash { 8 } else { 4 });
    }
}

#[test]
fn pareidolia_connects_face_scoring_retriggers_and_transformation() {
    let mut state = state_with(&[
        JokerKind::Pareidolia,
        JokerKind::ScaryFace,
        JokerKind::SockAndBuskin,
        JokerKind::Photograph,
    ]);
    let score = play(&mut state, &[card(Rank::Two, Suit::Clubs)]);
    assert_eq!((score.chips, score.mult), (69, 4.0));
    state.jokers.get_mut(0).unwrap().set_debuffed(true);
    assert_eq!((play(&mut state, &[card(Rank::Two, Suit::Clubs)]).chips), 7);
    let mut state = state_with(&[
        JokerKind::Pareidolia,
        JokerKind::MidasMask,
        JokerKind::GoldenTicket,
    ]);
    let before = state.money;
    play(&mut state, &[card(Rank::Two, Suit::Clubs)]);
    assert_eq!(state.money, before + 4);
    assert_eq!(
        state.deck.discarded_cards()[0].enhancement(),
        Enhancement::Gold
    );
}

#[test]
fn pareidolia_applies_to_discard_economy_and_ride_the_bus_updates() {
    let mut state = state_with(&[JokerKind::Pareidolia, JokerKind::Faceless]);
    let mut hand = Hand::new();
    for rank in [Rank::Two, Rank::Three, Rank::Four] {
        hand.add_card(card(rank, Suit::Clubs));
    }
    let before = state.money;
    state
        .discard_selected_cards(&mut hand, vec![0, 1, 2])
        .unwrap();
    assert_eq!(state.money, before + 5);

    let mut state = state_with(&[JokerKind::RideTheBus]);
    play(&mut state, &[card(Rank::Two, Suit::Clubs)]);
    state.jokers.add(Joker::create_joker(JokerKind::Pareidolia));
    play(&mut state, &[card(Rank::Two, Suit::Clubs)]);
    state.sell_joker(1).unwrap();
    assert_eq!(
        play(&mut state, &[card(Rank::Two, Suit::Clubs)]).mult,
        1.0,
        "Pareidolia reset the accumulated Mult"
    );
}

#[test]
fn pareidolia_changes_plant_debuffs_and_mark_draws() {
    let mut state = state_with(&[JokerKind::Pareidolia, JokerKind::SmileyFace]);
    state.begin_blind_as(BlindKind::Boss(BossBlinds::Plante));
    let score = play(&mut state, &[card(Rank::Two, Suit::Clubs)]);
    assert_eq!((score.chips, score.mult), (5, 1.0));
    let mut state = state_with(&[JokerKind::Pareidolia]);
    state.begin_blind_as(BlindKind::Boss(BossBlinds::Mark));
    let mut hand = Hand::new();
    state.deal_cards(&mut hand);
    assert!(hand.cards().iter().all(|card| card.is_face_down()));
    state.jokers.get_mut(0).unwrap().set_debuffed(true);
    let mut hand = Hand::new();
    state.deal_cards(&mut hand);
    assert!(
        hand.cards()
            .iter()
            .any(|card| !card.is_face_down() && !card.is_face_card())
    );
}

#[test]
fn shortcut_allows_repeated_single_gaps_but_not_large_gaps_or_ace_wrap() {
    for (ranks, expected) in [
        (
            [Rank::Two, Rank::Four, Rank::Six, Rank::Eight, Rank::Ten],
            PokerHand::Straight,
        ),
        (
            [Rank::Ace, Rank::Three, Rank::Five, Rank::Seven, Rank::Nine],
            PokerHand::Straight,
        ),
        (
            [Rank::Ace, Rank::Ten, Rank::Jack, Rank::Queen, Rank::King],
            PokerHand::Straight,
        ),
        (
            [Rank::Two, Rank::Five, Rank::Seven, Rank::Nine, Rank::Jack],
            PokerHand::HighCard,
        ),
        (
            [Rank::Queen, Rank::King, Rank::Ace, Rank::Two, Rank::Three],
            PokerHand::HighCard,
        ),
    ] {
        let cards = std::array::from_fn::<_, 5, _>(|index| {
            card(
                ranks[index],
                [
                    Suit::Hearts,
                    Suit::Clubs,
                    Suit::Diamonds,
                    Suit::Spades,
                    Suit::Hearts,
                ][index],
            )
        });
        let mut state = state_with(&[JokerKind::Shortcut]);
        assert_eq!(play(&mut state, &cards).hand, expected, "{ranks:?}");
    }
    let cards = [Rank::Two, Rank::Four, Rank::Six, Rank::Eight, Rank::Ten]
        .map(|rank| card(rank, Suit::Hearts));
    let mut state = state_with(&[JokerKind::Shortcut]);
    assert_eq!(play(&mut state, &cards).hand, PokerHand::StraightFlush);
    state.jokers.get_mut(0).unwrap().set_debuffed(true);
    assert_eq!(play(&mut state, &cards).hand, PokerHand::Flush);
}

#[test]
fn smeared_and_shortcut_combine_for_straight_flush_and_do_not_change_ranks() {
    let cards = std::array::from_fn::<_, 5, _>(|index| {
        card(
            [Rank::Two, Rank::Four, Rank::Six, Rank::Eight, Rank::Ten][index],
            if index % 2 == 0 {
                Suit::Hearts
            } else {
                Suit::Diamonds
            },
        )
    });
    let mut state = state_with(&[JokerKind::Shortcut, JokerKind::Smeared]);
    assert_eq!(play(&mut state, &cards).hand, PokerHand::StraightFlush);
    state.jokers.get_mut(1).unwrap().set_debuffed(true);
    assert_eq!(play(&mut state, &cards).hand, PokerHand::Straight);
    state.jokers.get_mut(0).unwrap().set_debuffed(true);
    assert_eq!(play(&mut state, &cards).hand, PokerHand::HighCard);
}

#[test]
fn smeared_connects_suit_scoring_economy_and_boss_debuffs() {
    let mut state = state_with(&[JokerKind::Smeared, JokerKind::Lusty, JokerKind::RoughGem]);
    let before = state.money;
    assert_eq!(
        play(&mut state, &[card(Rank::Two, Suit::Diamonds)]).mult,
        4.0
    );
    assert_eq!(state.money, before + 1);
    let before = state.money;
    assert_eq!(play(&mut state, &[card(Rank::Two, Suit::Hearts)]).mult, 4.0);
    assert_eq!(state.money, before + 1);
    assert_eq!(play(&mut state, &[card(Rank::Two, Suit::Spades)]).mult, 1.0);
    let mut state = state_with(&[JokerKind::Smeared, JokerKind::Arrowhead]);
    assert_eq!(play(&mut state, &[card(Rank::Two, Suit::Spades)]).chips, 57);
    state.begin_blind_as(BlindKind::Boss(BossBlinds::Club));
    assert_eq!(play(&mut state, &[card(Rank::Two, Suit::Spades)]).chips, 5);
}

#[test]
fn smeared_flower_pot_requires_two_cards_of_each_colour() {
    for (suits, expected_factor) in [
        (
            [
                Suit::Hearts,
                Suit::Hearts,
                Suit::Spades,
                Suit::Spades,
                Suit::Hearts,
            ],
            3.0,
        ),
        (
            [
                Suit::Hearts,
                Suit::Hearts,
                Suit::Hearts,
                Suit::Hearts,
                Suit::Spades,
            ],
            1.0,
        ),
    ] {
        let cards = std::array::from_fn::<_, 5, _>(|index| {
            card(
                [Rank::Two, Rank::Two, Rank::Two, Rank::Three, Rank::Three][index],
                suits[index],
            )
        });
        let mut state = state_with(&[JokerKind::Smeared, JokerKind::FlowerPot]);
        let result = play(&mut state, &cards);
        let base = state.hand_score(result.hand).mult as f32;
        assert_eq!(result.mult / base, expected_factor);
    }
}

#[test]
fn oops_all_sixes_stacks_and_debuffs_disable_probability_boosts() {
    let cards = [card(Rank::Two, Suit::Hearts)];
    for seed in 0..32 {
        let mut state = GameState::new(seed);
        state.begin_blind_as(BlindKind::Small);
        state.jokers.add(Joker::create_joker(JokerKind::Bloodstone));
        state.jokers.add(Joker::create_joker(JokerKind::OopsAll6s));
        assert_eq!(play(&mut state, &cards).mult, 1.5);
    }
    // Business Card is 1/2: one active Oops guarantees its payout.
    let mut state = state_with(&[JokerKind::BusinessCard, JokerKind::OopsAll6s]);
    let before = state.money;
    play(&mut state, &[card(Rank::King, Suit::Hearts)]);
    assert_eq!(state.money, before + 2);
    state.jokers.get_mut(1).unwrap().set_debuffed(true);
    assert_eq!(state.jokers.modifiers().probability_multiplier, 1.0);
}

#[test]
fn oops_all_sixes_doubles_wheel_of_fortune_and_caps_at_certainty() {
    for count in [0, 1, 2] {
        let mut successes = 0;
        for seed in 0..256 {
            let mut state = GameState::new(seed);
            state.jokers.add(Joker::create_joker(JokerKind::Joker));
            for _ in 0..count {
                state.jokers.add(Joker::create_joker(JokerKind::OopsAll6s));
            }
            state
                .add_consumable(Consumable::Tarot(Tarot::WheelOfFortune))
                .unwrap();
            state
                .use_consumable(0, &mut Hand::new(), ConsumableTarget::default())
                .unwrap();
            successes += usize::from(
                state
                    .jokers
                    .as_slice()
                    .iter()
                    .any(|joker| joker.edition() != JokerEdition::None),
            );
        }
        match count {
            0 => assert!((40..90).contains(&successes)),
            1 => assert!((100..160).contains(&successes)),
            2 => assert_eq!(successes, 256),
            _ => unreachable!(),
        }
    }
}

#[test]
fn oops_all_sixes_affects_wheel_blind_but_does_not_copy_through_blueprint() {
    let mut state = state_with(&[JokerKind::OopsAll6s, JokerKind::Blueprint]);
    // Retarget Blueprint to an incompatible Oops; it must not add another factor.
    state.jokers.as_mut_slice().swap(0, 1);
    assert_eq!(state.jokers.modifiers().probability_multiplier, 2.0);
    state.jokers.add(Joker::create_joker(JokerKind::OopsAll6s));
    state.jokers.add(Joker::create_joker(JokerKind::OopsAll6s));
    state.begin_blind_as(BlindKind::Boss(BossBlinds::Wheel));
    let mut hand = Hand::new();
    state.deal_cards(&mut hand);
    assert!(hand.cards().iter().all(|card| card.is_face_down()));
}

#[test]
fn astronomer_prices_existing_and_new_planet_offers_and_celestial_packs() {
    let mut state = state_with(&[JokerKind::Astronomer]);
    state.money = 0;
    let mut shop = ShopState::new();
    shop.add_item(random_base_item(ShopOfferKind::Planet, &mut state));
    shop.add_pack(ShopPack::new(
        BoosterPackKind::Celestial,
        BoosterPackSize::Mega,
        8,
    ))
    .unwrap();
    assert_eq!(shop.items[0].price(&state), 0);
    assert_eq!(shop.packs[0].price(&state), 0);
    shop.buy_item(0, &mut state).unwrap();
    shop.buy_and_open_pack(0, &mut state).unwrap();
    assert_eq!(state.money, 0);
    assert_eq!(state.consumables.len(), 1);
    assert_eq!(state.consumables[0].sell_value(), 1);
}

#[test]
fn astronomer_prices_restore_when_debuffed_or_sold_and_do_not_bypass_capacity() {
    let mut state = state_with(&[JokerKind::Astronomer]);
    let mut shop = ShopState::new();
    shop.add_item(ShopItem::Consumable {
        consumable: Consumable::Planet(Planet::Pluto),
        price: 3,
    });
    shop.add_item(ShopItem::Consumable {
        consumable: Consumable::Tarot(Tarot::Fool),
        price: 3,
    });
    shop.add_pack(ShopPack::new(
        BoosterPackKind::Arcana,
        BoosterPackSize::Normal,
        4,
    ))
    .unwrap();
    assert_eq!(shop.items[1].price(&state), 3);
    assert_eq!(shop.packs[0].price(&state), 4);
    state.jokers.get_mut(0).unwrap().set_debuffed(true);
    assert_eq!(shop.items[0].price(&state), 3);
    state.money = 0;
    assert_eq!(
        shop.buy_item(0, &mut state),
        Err(ShopError::InsufficientFunds)
    );
    state.jokers.get_mut(0).unwrap().set_debuffed(false);
    state
        .add_consumable(Consumable::Tarot(Tarot::Fool))
        .unwrap();
    state
        .add_consumable(Consumable::Tarot(Tarot::Magician))
        .unwrap();
    assert_eq!(shop.buy_item(0, &mut state), Err(ShopError::InventoryFull));
    assert_eq!(shop.items.len(), 2);
    state.sell_joker(0).unwrap();
    assert_eq!(shop.items[0].price(&state), 3);
}

#[test]
fn showman_allows_owned_shop_consumables_but_not_repeated_vouchers() {
    use crate::core::enums::Spectral;
    for (kind, owned) in [
        (ShopOfferKind::Tarot, Consumable::Tarot(Tarot::Fool)),
        (ShopOfferKind::Planet, Consumable::Planet(Planet::Pluto)),
        (
            ShopOfferKind::Spectral,
            Consumable::Spectral(Spectral::Aura),
        ),
    ] {
        let mut duplicates = 0;
        for seed in 0..256 {
            for debuffed in [false, true] {
                let mut state = GameState::new(seed);
                let mut showman = Joker::create_joker(JokerKind::Showman);
                showman.set_debuffed(debuffed);
                state.jokers.add(showman);
                state.add_consumable(owned).unwrap();
                let offer = random_base_item(kind, &mut state);
                let duplicate =
                    matches!(offer, ShopItem::Consumable { consumable, .. } if consumable == owned);
                if debuffed {
                    assert!(!duplicate, "{kind:?}, seed={seed}");
                } else {
                    duplicates += usize::from(duplicate);
                }
            }
        }
        assert!(duplicates > 0, "{kind:?}");
    }
    let mut state = state_with(&[JokerKind::Showman]);
    state.redeem_voucher(Vouchers::Overstock).unwrap();
    assert!(!state.can_redeem_voucher(Vouchers::Overstock));
    assert!(state.can_redeem_voucher(Vouchers::OverstockPlus));
}

#[test]
fn pareidolia_applies_to_held_economy_and_plant_blocks_it() {
    for plant in [false, true] {
        let mut state = state_with(&[
            JokerKind::Pareidolia,
            JokerKind::ReservedParking,
            JokerKind::OopsAll6s,
            JokerKind::Mime,
        ]);
        if plant {
            state.begin_blind_as(BlindKind::Boss(BossBlinds::Plante));
        }
        let mut hand = Hand::new();
        hand.add_card(card(Rank::Two, Suit::Clubs));
        hand.add_card(card(Rank::Three, Suit::Hearts));
        let before = state.money;
        state.play_hand(&mut hand, vec![0]).unwrap();
        assert_eq!(state.money - before, if plant { 0 } else { 2 });
    }
    let mut state = state_with(&[
        JokerKind::Pareidolia,
        JokerKind::BusinessCard,
        JokerKind::OopsAll6s,
    ]);
    state.begin_blind_as(BlindKind::Boss(BossBlinds::Plante));
    let before = state.money;
    play(&mut state, &[card(Rank::Two, Suit::Hearts)]);
    assert_eq!(state.money, before);
}

#[test]
fn shortcut_is_used_by_burnt_jokers_discard_hand_detection() {
    for debuffed in [false, true] {
        let mut state = state_with(&[JokerKind::Shortcut, JokerKind::BurntJoker]);
        state.jokers.get_mut(0).unwrap().set_debuffed(debuffed);
        let before = state.hand_score(PokerHand::Straight).chips;
        let mut hand = Hand::new();
        for (rank, suit) in [Rank::Two, Rank::Four, Rank::Six, Rank::Eight, Rank::Ten]
            .iter()
            .zip([
                Suit::Hearts,
                Suit::Clubs,
                Suit::Diamonds,
                Suit::Spades,
                Suit::Hearts,
            ])
        {
            hand.add_card(card(*rank, suit));
        }
        state
            .discard_selected_cards(&mut hand, vec![0, 1, 2, 3, 4])
            .unwrap();
        assert_eq!(
            state.hand_score(PokerHand::Straight).chips,
            before + if debuffed { 0 } else { 30 }
        );
    }
}

#[test]
fn smeared_seeing_double_requires_black_and_red_and_rank_based_hands_stay_valid() {
    for (suits, expected_factor) in [
        ([Suit::Spades, Suit::Hearts], 2.0),
        ([Suit::Spades, Suit::Spades], 1.0),
        ([Suit::Hearts, Suit::Hearts], 1.0),
    ] {
        let mut state = state_with(&[JokerKind::Smeared, JokerKind::SeeingDouble]);
        let result = play(
            &mut state,
            &[card(Rank::Two, suits[0]), card(Rank::Two, suits[1])],
        );
        assert_eq!(result.hand, PokerHand::Pair);
        assert_eq!(result.mult, 2.0 * expected_factor);
    }
    let mut state = state_with(&[JokerKind::Smeared]);
    let cards = std::array::from_fn::<_, 5, _>(|index| {
        card(
            Rank::Two,
            if index % 2 == 0 {
                Suit::Hearts
            } else {
                Suit::Diamonds
            },
        )
    });
    assert_eq!(play(&mut state, &cards).hand, PokerHand::FlushFive);
    state.sell_joker(0).unwrap();
    assert_eq!(play(&mut state, &cards).hand, PokerHand::FiveOfAKind);
}

#[test]
fn showman_enables_duplicate_joker_offers_but_does_not_control_playing_card_duplicates() {
    let mut duplicates = 0;
    for seed in 0..256 {
        for debuffed in [false, true] {
            let mut state = GameState::new(seed);
            let mut showman = Joker::create_joker(JokerKind::Showman);
            showman.set_debuffed(debuffed);
            state.jokers.add(showman);
            state.jokers.add(Joker::create_joker(JokerKind::Joker));
            let ShopItem::Joker { joker, .. } = random_base_item(ShopOfferKind::Joker, &mut state)
            else {
                unreachable!()
            };
            let duplicate = matches!(joker.kind(), JokerKind::Joker | JokerKind::Showman);
            if debuffed {
                assert!(!duplicate);
            } else {
                duplicates += usize::from(duplicate);
            }
        }
    }
    assert!(duplicates > 0);
    for showman in [false, true] {
        let mut state = state_with(if showman { &[JokerKind::Showman] } else { &[] });
        let mut shop = ShopState::new();
        state.money = 10;
        for _ in 0..2 {
            shop.add_item(ShopItem::PlayingCard {
                card: card(Rank::Two, Suit::Hearts),
                price: 1,
            });
        }
        let before = state.deck_size();
        shop.buy_item(0, &mut state).unwrap();
        shop.buy_item(0, &mut state).unwrap();
        assert_eq!(state.deck_size(), before + 2);
    }
}

#[test]
fn sold_showman_restores_pack_duplicate_exclusion() {
    use crate::core::pack::{consumable_available_in_pack, joker_available_in_pack};
    let mut state = state_with(&[JokerKind::Showman, JokerKind::Joker]);
    state
        .add_consumable(Consumable::Planet(Planet::Pluto))
        .unwrap();
    assert!(joker_available_in_pack(&state, JokerKind::Joker, &[]));
    assert!(consumable_available_in_pack(
        &state,
        Consumable::Planet(Planet::Pluto),
        &[]
    ));
    state.sell_joker(0).unwrap();
    assert!(!joker_available_in_pack(&state, JokerKind::Joker, &[]));
    assert!(!consumable_available_in_pack(
        &state,
        Consumable::Planet(Planet::Pluto),
        &[]
    ));
}

#[test]
fn selling_face_and_suit_modifiers_recalculates_debuffs_on_already_held_cards() {
    for (modifier, boss, rank, suit) in [
        (
            JokerKind::Pareidolia,
            BossBlinds::Plante,
            Rank::Two,
            Suit::Clubs,
        ),
        (
            JokerKind::Smeared,
            BossBlinds::Club,
            Rank::Two,
            Suit::Spades,
        ),
    ] {
        let mut state = state_with(&[modifier]);
        state.begin_blind_as(BlindKind::Boss(boss));
        let mut hand = Hand::new();
        hand.add_card(card(rank, suit));
        hand.add_card(card(rank, suit));
        state.play_hand(&mut hand, vec![0]).unwrap();
        assert!(hand.cards()[0].is_debuffed());
        state.sell_joker(0).unwrap();
        let score = state.play_hand(&mut hand, vec![0]).unwrap();
        assert_eq!(score.chips, 7, "{modifier:?}");
    }
}

#[test]
fn blueprint_cannot_double_oops_probability_and_debuffs_restore_base_odds() {
    for debuffed in [false, true] {
        let mut successes = 0;
        for seed in 0..256 {
            let mut state = GameState::new(seed);
            state.jokers.add(Joker::create_joker(JokerKind::Blueprint));
            let mut oops = Joker::create_joker(JokerKind::OopsAll6s);
            oops.set_debuffed(debuffed);
            state.jokers.add(oops);
            state
                .add_consumable(Consumable::Tarot(Tarot::WheelOfFortune))
                .unwrap();
            state
                .use_consumable(0, &mut Hand::new(), ConsumableTarget::default())
                .unwrap();
            successes += usize::from(
                state
                    .jokers
                    .as_slice()
                    .iter()
                    .any(|joker| joker.edition() != JokerEdition::None),
            );
        }
        let expected = if debuffed { 40..90 } else { 100..160 };
        assert!(
            expected.contains(&successes),
            "debuffed={debuffed}, successes={successes}"
        );
    }
}

#[test]
fn astronomer_free_price_overrides_discounts_and_restores_existing_pack_prices() {
    let mut state = state_with(&[JokerKind::Astronomer]);
    state.shop_discount_percent = 50;
    let pack = ShopPack::new(BoosterPackKind::Celestial, BoosterPackSize::Mega, 8);
    let planet = ShopItem::Consumable {
        consumable: Consumable::Planet(Planet::Pluto),
        price: 3,
    };
    assert_eq!(pack.price(&state), 0);
    assert_eq!(planet.price(&state), 0);
    state.jokers.get_mut(0).unwrap().set_debuffed(true);
    assert_eq!(pack.price(&state), 4);
    assert_eq!(planet.price(&state), 1);
}

#[test]
fn splash_controls_whether_excluded_kickers_can_satisfy_suit_jokers() {
    for (joker, cards, factor) in [
        (
            JokerKind::FlowerPot,
            [
                card(Rank::Two, Suit::Hearts),
                card(Rank::Two, Suit::Hearts),
                card(Rank::Two, Suit::Hearts),
                card(Rank::Two, Suit::Spades),
                card(Rank::Jack, Suit::Spades),
            ],
            3.0,
        ),
        (
            JokerKind::SeeingDouble,
            [
                card(Rank::Two, Suit::Hearts),
                card(Rank::Two, Suit::Hearts),
                card(Rank::Two, Suit::Hearts),
                card(Rank::Two, Suit::Hearts),
                card(Rank::Jack, Suit::Spades),
            ],
            2.0,
        ),
    ] {
        for splash in [false, true] {
            let mut state = state_with(&[JokerKind::Smeared, joker]);
            if splash {
                state.jokers.add(Joker::create_joker(JokerKind::Splash));
            }
            let result = play(&mut state, &cards);
            assert_eq!(result.hand, PokerHand::FourOfAKind);
            assert_eq!(
                result.mult,
                7.0 * if splash { factor } else { 1.0 },
                "{joker:?}, splash={splash}"
            );
        }
    }
}

#[test]
fn ride_the_bus_ignores_excluded_and_debuffed_faces() {
    let mut state = state_with(&[JokerKind::RideTheBus]);
    play(
        &mut state,
        &[
            card(Rank::Two, Suit::Hearts),
            card(Rank::Two, Suit::Clubs),
            card(Rank::Jack, Suit::Spades),
        ],
    );
    assert_eq!(play(&mut state, &[card(Rank::Two, Suit::Clubs)]).mult, 2.0);

    let mut state = state_with(&[JokerKind::Pareidolia, JokerKind::RideTheBus]);
    state.begin_blind_as(BlindKind::Boss(BossBlinds::Plante));
    play(&mut state, &[card(Rank::Two, Suit::Clubs)]);
    state.jokers.get_mut(0).unwrap().set_debuffed(true);
    assert_eq!(play(&mut state, &[card(Rank::Two, Suit::Clubs)]).mult, 2.0);
}

#[test]
fn pareidolia_does_not_activate_transformations_or_discard_economy_on_plant_debuffs() {
    let mut state = state_with(&[
        JokerKind::Pareidolia,
        JokerKind::MidasMask,
        JokerKind::Faceless,
    ]);
    state.begin_blind_as(BlindKind::Boss(BossBlinds::Plante));
    play(&mut state, &[card(Rank::Two, Suit::Hearts)]);
    assert_eq!(
        state.deck.discarded_cards()[0].enhancement(),
        Enhancement::None
    );
    let mut hand = Hand::new();
    state.deal_cards(&mut hand);
    assert!(hand.cards().iter().all(|card| card.is_debuffed()));
    let before = state.money;
    state
        .discard_selected_cards(&mut hand, vec![0, 1, 2])
        .unwrap();
    assert_eq!(state.money, before);
}

#[test]
fn pareidolia_does_not_change_ranks_or_rank_specific_joker_conditions() {
    let mut state = state_with(&[
        JokerKind::Pareidolia,
        JokerKind::Baron,
        JokerKind::Triboulet,
    ]);
    let mut hand = Hand::new();
    hand.add_card(card(Rank::Two, Suit::Clubs));
    hand.add_card(card(Rank::Three, Suit::Hearts));
    hand.add_card(card(Rank::Four, Suit::Spades));
    let score = state.play_hand(&mut hand, vec![0, 1]).unwrap();
    assert_eq!(score.hand, PokerHand::HighCard);
    assert_eq!((score.chips, score.mult), (8, 1.0));
    assert_eq!(state.deck.discarded_cards()[0].rank(), Rank::Two);
    assert_eq!(state.deck.discarded_cards()[1].rank(), Rank::Three);
}

#[test]
fn oops_does_not_scale_misprints_unlisted_random_value_selection() {
    for seed in 0..32 {
        let mut outcomes = Vec::new();
        for oops in [false, true] {
            let mut state = GameState::new(seed);
            state.begin_blind_as(BlindKind::Small);
            state.jokers.add(Joker::create_joker(JokerKind::Misprint));
            if oops {
                state.jokers.add(Joker::create_joker(JokerKind::OopsAll6s));
            }
            outcomes.push(play(&mut state, &[card(Rank::Two, Suit::Clubs)]).mult);
        }
        assert_eq!(outcomes[0], outcomes[1], "seed={seed}");
    }
}
