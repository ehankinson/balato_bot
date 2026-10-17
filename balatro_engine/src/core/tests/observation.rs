use super::*;
use crate::core::enums::{Rank, Suit};
use crate::core::joker_types::{JokerEdition, JokerKind};
use crate::core::observation::{Observation, ObservationError, Table};
use crate::core::pack::{BoosterPackKind, BoosterPackSize, PackOption, ShopPack};
use crate::core::shop::{ShopItem, ShopVoucher};
use rand::Rng;

fn l(value: f64) -> f32 {
    (value.signum() * value.abs().ln_1p()) as f32
}

fn playing(seed: u64) -> Game {
    let mut game = Game::new(seed);
    game.apply_action(GameAction::SelectBlind(BlindKind::Small))
        .unwrap();
    game.state.blind.score_requirement = u64::MAX;
    game
}

fn boss_game(boss: BossBlinds) -> Game {
    let mut game = Game::new(503);
    game.blind_stage = BlindStage::Boss;
    game.state.blind_selector.set_boss(boss);
    game.apply_action(GameAction::SelectBlind(game.available_blind()))
        .unwrap();
    game.state.blind.score_requirement = u64::MAX;
    game
}

fn open_pack(game: &mut Game, kind: BoosterPackKind, size: BoosterPackSize) {
    game.state.money = 100;
    game.state.shop.packs = vec![ShopPack::new(kind, size, 0)];
    game.apply_action(GameAction::BuyPack(0)).unwrap();
}

fn shapes<const C: usize, const N: usize, const F: usize>(table: &Table<C, N, F>) {
    assert_eq!(table.cat.len(), table.len());
    assert_eq!(table.num.len(), table.len());
    assert_eq!(table.flags.len(), table.len());
    assert_eq!(table.visible.len(), table.len() * (C + N + F));
    assert!(table.num.iter().flatten().all(|value| value.is_finite()));
    for index in 0..table.len() {
        if !table.present[index] {
            assert_eq!(table.cat[index], [0; C]);
            assert_eq!(table.num[index], [0.0; N]);
            assert_eq!(table.flags[index], [false; F]);
            assert!(
                table.visible[index * (C + N + F)..(index + 1) * (C + N + F)]
                    .iter()
                    .all(|visible| !visible)
            );
        }
    }
}

fn all_shapes(observation: &Observation) {
    assert_eq!(observation.schema_version, [1]);
    shapes(&observation.jokers);
    shapes(&observation.joker_values);
    shapes(&observation.consumables);
    shapes(&observation.last_consumable);
    shapes(&observation.deck_catalogue);
    shapes(&observation.poker_hands);
    shapes(&observation.vouchers);
    shapes(&observation.tags);
    shapes(&observation.blind_offers);
    shapes(&observation.hand);
    shapes(&observation.shop_items);
    shapes(&observation.shop_vouchers);
    shapes(&observation.shop_packs);
    shapes(&observation.pack_options);
    assert_eq!(observation.poker_hand_counts.len(), 12);
    assert_eq!(
        observation.deck_catalogue.len(),
        observation.deck_multiplicity.len()
    );
    assert_eq!(
        observation.deck_multiplicity.iter().sum::<i64>(),
        observation.run.counts[7]
    );
}

#[test]
fn initial_observation_has_typed_persistent_data_and_absent_sections() {
    let game = Game::new(501);
    let observation = game.observe().unwrap();
    all_shapes(&observation);
    assert_eq!(observation.sections, [true, false, false, false, false]);
    assert_eq!(observation.run.cat, [0, 1, 1, 0, 0]);
    assert_eq!(
        observation.run.counts,
        [1, 1, 8, 5, 2, 0, 0, 52, 52, 0, 0, 0, 0, 0]
    );
    assert_eq!(observation.run.num, [l(4.0), l(5.0), 0.0]);
    assert!(!observation.run.visible[21]);
    assert_eq!(observation.deck_catalogue.len(), 52);
    assert!(
        observation
            .deck_multiplicity
            .iter()
            .all(|count| *count == 1)
    );
    assert_eq!(observation.deck_catalogue.cat[0], [2, 0, 0, 0, 0]);
    assert_eq!(observation.deck_catalogue.num[0], [0.0, 0.0, l(1.0)]);
    assert_eq!(
        observation
            .poker_hands
            .cat
            .iter()
            .map(|row| row[0])
            .collect::<Vec<_>>(),
        (1..=12).collect::<Vec<_>>()
    );
    assert_eq!(observation.poker_hand_counts, vec![[1, 0]; 12]);
    assert_eq!(observation.blind_offers.num[0], [l(300.0), l(3.0)]);
    assert_eq!(observation.blind_offers.num[1], [l(450.0), l(4.0)]);
    assert_eq!(
        observation.blind_offers.flags,
        [[true, true], [false, false], [false, false]]
    );
    assert!(observation.hand.present.is_empty());
    assert_eq!(observation.blind.visible, [false; 14]);
    assert_eq!(observation.pack.visible, [false; 4]);
    assert_eq!(observation.shop.visible, [false]);
    assert_eq!(observation.action_kinds, game.action_kinds());
}

#[test]
fn deck_and_stake_categories_and_initial_compositions_are_not_policy_sized() {
    for deck in [
        Decks::Red,
        Decks::Blue,
        Decks::Yellow,
        Decks::Green,
        Decks::Black,
        Decks::Magic,
        Decks::Nebula,
        Decks::Ghost,
        Decks::Abandoned,
        Decks::Checkered,
        Decks::Zodiac,
        Decks::Painted,
        Decks::Plasma,
        Decks::Erratic,
    ] {
        for stake in [Stakes::White, Stakes::Blue, Stakes::Gold] {
            let game = Game::with_rules(502, deck, stake);
            let observation = game.observe().unwrap();
            all_shapes(&observation);
            assert_eq!(observation.run.cat[1], deck as i32);
            assert_eq!(observation.run.cat[2], stake as i32);
            let size = if deck == Decks::Abandoned { 40 } else { 52 };
            assert_eq!(observation.run.counts[7], size);
            if deck == Decks::Checkered {
                assert_eq!(observation.deck_catalogue.len(), 26);
                assert!(
                    observation
                        .deck_multiplicity
                        .iter()
                        .all(|count| *count == 2)
                );
            }
        }
    }
}

#[test]
fn playing_observation_exposes_public_score_history_and_zero_valued_properties() {
    let mut game = playing(502);
    let before = game.observe().unwrap();
    all_shapes(&before);
    assert_eq!(before.sections, [false, true, false, false, true]);
    assert_eq!(before.hand.len(), 8);
    assert!(before.hand.present.iter().all(|present| *present));
    assert!(before.hand.visible.iter().all(|visible| *visible));
    assert_eq!(before.run.counts[8..11], [44, 8, 0]);
    assert!(!before.run.visible[3]);
    assert!(before.blind_offers.present.iter().all(|present| !present));
    game.apply_action(GameAction::PlayCards(vec![0])).unwrap();
    let after = game.observe().unwrap();
    assert_eq!(after.blind.counts[..4], [3, 4, 1, 0]);
    assert!(after.blind.flags[1]);
    assert!(after.blind.played_types[0]);
    assert_eq!(after.blind.cat[3], 1);
    assert_eq!(after.poker_hand_counts[0], [1, 1]);
    assert!(after.blind.num[0] > 0.0);
    assert_eq!(
        before.blind.counts[..4],
        [4, 4, 0, 0],
        "old observations stay independently owned"
    );
    assert_eq!(before.blind.num[0], 0.0);
}

#[test]
fn face_down_cards_are_present_with_only_back_and_forced_selection_visible() {
    let mut game = boss_game(BossBlinds::CeruleanBell);
    let index = game.state.blind.forced_card_index.unwrap();
    game.hand.card_mut(index).unwrap().set_face_down(true);
    game.hand.card_mut(index).unwrap().set_debuffed(true);
    let observation = game.observe().unwrap();
    assert!(observation.hand.present[index]);
    assert_eq!(observation.hand.cat[index], [0; 5]);
    assert_eq!(observation.hand.num[index], [0.0; 3]);
    assert_eq!(observation.hand.flags[index], [true, false, true]);
    assert_eq!(
        &observation.hand.visible[index * 11..index * 11 + 11],
        &[
            false, false, false, false, false, false, false, false, true, false, true
        ]
    );
    assert_eq!(observation.blind.counts[4], index as i64);
    assert!(observation.blind.visible[8]);
    assert!(observation.action_kinds[2]);
}

#[test]
fn hidden_card_mutations_are_frozen_in_inspection_until_revelation() {
    let mut game = playing(504);
    game.hand.card_mut(0).unwrap().set_face_down(true);
    let before = game.observe().unwrap();
    let card = game.hand.card_mut(0).unwrap();
    card.set_rank(Rank::Ace);
    card.set_suit(Suit::Hearts);
    card.set_enhancement(Enhancement::Glass);
    card.set_edition(Edition::Polychrome);
    card.set_seal(Seal::Gold);
    card.scoring_values_mut().chips += 999;
    card.mark_played(1);
    card.set_debuffed(true);
    assert_eq!(game.observe().unwrap(), before);
    game.hand.card_mut(0).unwrap().set_face_down(false);
    let revealed = game.observe().unwrap();
    assert_eq!(revealed.hand.cat[0], [14, 0, 7, 3, 1]);
    assert_ne!(revealed.deck_catalogue, before.deck_catalogue);
}

#[test]
fn anonymous_hidden_destruction_cannot_identify_the_card_through_histograms() {
    let mut left = playing(505);
    let mut right = playing(505);
    // Exchange a concealed hand occurrence with an undealt occurrence while
    // preserving the same public full-deck composition and all visible slots.
    let original = right.hand.take_cards();
    let mut cards = original;
    let drawn = right.state.deck.remove_card_at(0).unwrap();
    right.state.deck.add_card_to_deck(cards.remove(0));
    cards.insert(0, drawn);
    for card in cards {
        right.hand.add_card(card);
    }
    for game in [&mut left, &mut right] {
        game.hand.card_mut(0).unwrap().set_face_down(true);
        game.state
            .add_consumable(Consumable::Tarot(Tarot::HangedMan))
            .unwrap();
    }
    assert_eq!(left.observe().unwrap(), right.observe().unwrap());
    for game in [&mut left, &mut right] {
        game.apply_action(GameAction::UseConsumable {
            index: 0,
            target: ConsumableTarget::hand(vec![0]),
        })
        .unwrap();
    }
    let after = left.observe().unwrap();
    assert_eq!(after, right.observe().unwrap());
    all_shapes(&after);
    assert!(after.deck_catalogue.present[0]);
    assert!(
        !after.deck_catalogue.visible[..8]
            .iter()
            .any(|visible| *visible)
    );
    assert_eq!(after.deck_multiplicity[0], 44);
    assert_eq!(after.run.counts[7], 51);
}

#[test]
fn deck_inspection_is_independent_of_draw_order_rng_and_private_history() {
    let mut game = playing(506);
    let before = game.observe().unwrap();
    game.state.deck.shuffle(&mut game.state.rng);
    game.state.rng.next_u64();
    game.state.random_boss_blind();
    game.state.deck.card_mut(0).unwrap().mark_played(1);
    game.state.deck.card_mut(0).unwrap().set_debuffed(true);
    assert_eq!(game.observe().unwrap(), before);
}

#[test]
fn hidden_joker_properties_stickers_and_dynamic_values_do_not_leak() {
    let mut game = playing(507);
    let mut joker = Joker::create_joker(JokerKind::RedCard);
    joker.set_face_down(true);
    game.state.jokers.add(joker);
    let before = game.observe().unwrap();
    let joker = game.state.jokers.get_mut(0).unwrap();
    joker.set_eternal();
    joker.set_rental();
    joker.set_debuffed(true);
    joker.set_edition(JokerEdition::Foil);
    assert_eq!(game.observe().unwrap(), before);
    assert_eq!(before.jokers.cat[0], [0; 3]);
    assert_eq!(before.jokers.num[0], [0.0]);
    assert_eq!(before.jokers.flags[0], [true, false, false, false, false]);
    assert_eq!(
        before.jokers.visible,
        [false, false, false, false, true, false, false, false, false]
    );
    assert_eq!(before.joker_values.len(), 0);
    game.state.jokers.get_mut(0).unwrap().set_face_down(false);
    let after = game.observe().unwrap();
    assert_eq!(after.jokers.cat[0], [63, 0, 1]);
    assert!(after.jokers.flags[0][1]);
    assert!(after.jokers.flags[0][2]);
    assert!(after.jokers.flags[0][4]);
    assert!(after.joker_values.cat.iter().any(|row| *row == [0, 0, 2]));
}

#[test]
fn all_joker_categories_and_visible_zero_counters_have_stable_meanings() {
    for kind in JokerKind::ALL {
        let mut game = Game::new(508);
        game.state.jokers.add(Joker::create_joker(kind));
        let observation = game.observe().unwrap();
        all_shapes(&observation);
        assert_eq!(observation.jokers.cat[0][0], kind as i32);
        assert!(observation.jokers.visible.iter().all(|visible| *visible));
        assert!(
            observation
                .joker_values
                .visible
                .iter()
                .all(|visible| *visible)
        );
    }
    let mut game = Game::new(508);
    game.state
        .jokers
        .add(Joker::create_joker(JokerKind::Castle));
    let observation = game.observe().unwrap();
    let index = observation
        .joker_values
        .cat
        .iter()
        .position(|row| *row == [0, 0, 1])
        .unwrap();
    assert_eq!(observation.joker_values.num[index], [0.0]);
    assert!(observation.joker_values.visible[index * 4 + 3]);
}

#[test]
fn current_castle_target_comes_from_its_rule_owner_without_rolling() {
    let mut game = Game::new(527);
    game.state
        .jokers
        .add(Joker::create_joker(JokerKind::Castle));
    game.apply_action(GameAction::SelectBlind(BlindKind::Small))
        .unwrap();
    let rng = game.state.rng.get_word_pos();
    let observation = game.observe().unwrap();
    let index = observation
        .joker_values
        .cat
        .iter()
        .position(|row| *row == [0, 0, 8])
        .unwrap();
    assert_eq!(
        observation.joker_values.num[index][0],
        game.state.blind.target_suit.unwrap() as u8 as f32
    );
    assert_eq!(game.state.rng.get_word_pos(), rng);
}

#[test]
fn joker_value_progress_and_perishable_counts_update_at_their_boundaries() {
    let mut game = Game::new(509);
    let mut invisible = Joker::create_joker(JokerKind::Invisible);
    invisible.set_perishable();
    game.state.jokers.add(invisible);
    let before = game.observe().unwrap();
    assert!(before.joker_values.cat.iter().any(|row| *row == [0, 0, 18]));
    game.apply_action(GameAction::SelectBlind(BlindKind::Small))
        .unwrap();
    game.state.blind.score_requirement = 0;
    game.apply_action(GameAction::PlayCards(vec![0])).unwrap();
    let after = game.observe().unwrap();
    let round = after
        .joker_values
        .cat
        .iter()
        .position(|row| row[2] == 18)
        .unwrap();
    let perish = after
        .joker_values
        .cat
        .iter()
        .position(|row| row[2] == 20)
        .unwrap();
    assert_eq!(after.joker_values.num[round], [1.0]);
    assert_eq!(after.joker_values.num[perish], [4.0]);
}

#[test]
fn real_hand_levels_survive_saturated_scores_and_arm_has_a_level_one_floor() {
    let mut game = Game::new(510);
    game.state
        .add_consumable(Consumable::Planet(Planet::Pluto))
        .unwrap();
    game.apply_action(GameAction::UseConsumable {
        index: 0,
        target: ConsumableTarget::default(),
    })
    .unwrap();
    assert_eq!(game.observe().unwrap().poker_hand_counts[0], [2, 0]);
    game.state
        .upgrade_hand_score(PokerHand::HighCard, u16::MAX, u16::MAX);
    for _ in 0..3 {
        game.state.upgrade_hand_score(PokerHand::HighCard, 10, 1);
    }
    assert_eq!(game.observe().unwrap().poker_hand_counts[0][0], 6);
    assert_eq!(game.state.hand_score(PokerHand::HighCard).chips, u16::MAX);
    assert!(game.state.downgrade_hand_score(PokerHand::HighCard));
    assert_eq!(game.observe().unwrap().poker_hand_counts[0][0], 5);
    while game.state.downgrade_hand_score(PokerHand::HighCard) {}
    assert_eq!(game.state.hand_score(PokerHand::HighCard).level, 1);
    let score = game.state.hand_score(PokerHand::HighCard);
    assert!(!game.state.downgrade_hand_score(PokerHand::HighCard));
    assert_eq!(game.state.hand_score(PokerHand::HighCard), score);
}

#[test]
fn pending_tags_disappear_when_applied_instead_of_accumulating_history() {
    let mut game = Game::new(511);
    let mut tags = std::mem::take(&mut game.state.tags);
    for tag in [
        SkipTag::Investment,
        SkipTag::Juggle,
        SkipTag::Coupon,
        SkipTag::D6,
    ] {
        tags.gain(tag, &mut game.state);
    }
    tags.gain(SkipTag::Economy, &mut game.state);
    game.state.tags = tags;
    let before = game.observe().unwrap();
    assert_eq!(before.tags.cat, [[7, 0], [19, 0], [17, 0], [20, 0]]);
    assert_eq!(before.tags.num[0], [l(25.0)]);
    assert_eq!(before.tags.num[1], [l(3.0)]);
    game.apply_action(GameAction::SelectBlind(BlindKind::Small))
        .unwrap();
    assert_eq!(game.observe().unwrap().tags.cat, [[7, 0], [17, 0], [20, 0]]);
    game.state.blind.score_requirement = 0;
    game.apply_action(GameAction::PlayCards(vec![0])).unwrap();
    assert_eq!(game.observe().unwrap().tags.cat, [[7, 0]]);
    assert_eq!(game.state.shop.reroll_cost, 0);
    game.apply_action(GameAction::LeaveShop).unwrap();
    game.blind_stage = BlindStage::Boss;
    game.apply_action(GameAction::SelectBlind(game.available_blind()))
        .unwrap();
    game.state.blind.score_requirement = 0;
    game.apply_action(GameAction::PlayCards(vec![
        game.state.blind.forced_card_index.unwrap_or(0),
    ]))
    .unwrap();
    assert!(game.observe().unwrap().tags.cat.is_empty());
}

#[test]
fn double_tag_is_pending_until_gain_and_instant_tags_are_not_retained() {
    let mut game = Game::new(512);
    let mut tags = std::mem::take(&mut game.state.tags);
    tags.gain(SkipTag::Double, &mut game.state);
    game.state.tags = tags;
    assert_eq!(game.observe().unwrap().tags.cat, [[18, 0]]);
    let mut tags = std::mem::take(&mut game.state.tags);
    tags.gain(SkipTag::Investment, &mut game.state);
    tags.gain(SkipTag::Orbital, &mut game.state);
    game.state.tags = tags;
    assert_eq!(game.observe().unwrap().tags.cat, [[7, 0], [7, 0]]);
    assert!(
        game.observe()
            .unwrap()
            .poker_hand_counts
            .iter()
            .any(|row| row[0] == 4)
    );
}

#[test]
fn shop_union_fields_prices_and_voucher_indices_match_the_purchase_rules() {
    let mut game = Game::new(513);
    game.state.money = 100;
    game.state.redeem_voucher(Vouchers::ClearanceSale).unwrap();
    game.enter_shop().unwrap();
    let mut joker = Joker::create_joker(JokerKind::Joker);
    joker.set_edition(JokerEdition::Foil);
    let card = Card::new(
        Rank::Two,
        Suit::Hearts,
        Enhancement::Bonus,
        Edition::Holographic,
        Seal::Red,
    );
    game.state.shop.items = vec![
        ShopItem::Consumable {
            consumable: Consumable::Spectral(Spectral::Hex),
            price: 4,
        },
        ShopItem::Joker { joker, price: 6 },
        ShopItem::PlayingCard { card, price: 5 },
    ];
    game.state.shop.voucher = None;
    game.state.shop.extra_vouchers = vec![ShopVoucher {
        voucher: Vouchers::PaintBrush,
        price: 10,
    }];
    let before = game.observe().unwrap();
    all_shapes(&before);
    assert_eq!(before.sections, [false, false, true, false, false]);
    assert_eq!(before.shop_items.cat[0][..3], [2, 13, 3]);
    assert_eq!(before.shop_items.num[0][..2], [l(3.0), l(1.0)]);
    assert_eq!(before.shop_items.cat[1][8..], [1, 0]);
    assert_eq!(before.shop_items.cat[2][3..8], [2, 0, 3, 2, 4]);
    assert!(
        !before.shop_items.visible[2 * 20 + 1],
        "cards have no packed identity ID"
    );
    assert_eq!(before.shop_vouchers.present, [false, true]);
    assert_eq!(before.shop_vouchers.cat[1], [31]);
    assert_eq!(before.shop_vouchers.num[1], [l(7.0)]);
    let money = game.state.money;
    game.apply_action(GameAction::BuyShopItem(0)).unwrap();
    assert_eq!(money - game.state.money, 3);
    assert_eq!(game.observe().unwrap().consumables.num[0], [l(1.0)]);
    game.apply_action(GameAction::BuyVoucher(1)).unwrap();
    let after = game.observe().unwrap();
    assert_eq!(after.shop_vouchers.len(), 1);
    assert_eq!(after.shop_vouchers.present, [false]);
    assert_eq!(after.vouchers.cat, [[3], [31]]);
}

#[test]
fn free_prices_are_known_zero_and_owned_sell_values_do_not_use_free_purchase_prices() {
    let mut game = Game::new(514);
    game.state
        .jokers
        .add(Joker::create_joker(JokerKind::Astronomer));
    game.enter_shop().unwrap();
    game.state.shop.items = vec![ShopItem::Consumable {
        consumable: Consumable::Planet(Planet::Pluto),
        price: 3,
    }];
    game.state.shop.packs = vec![ShopPack::new(
        BoosterPackKind::Celestial,
        BoosterPackSize::Normal,
        4,
    )];
    let before = game.observe().unwrap();
    assert_eq!(before.shop_items.num[0][0], 0.0);
    assert!(before.shop_items.visible[10]);
    assert_eq!(before.shop_items.num[0][1], l(1.0));
    assert_eq!(before.shop_packs.num[0], [0.0]);
    assert!(before.shop_packs.visible[2]);
    game.apply_action(GameAction::BuyShopItem(0)).unwrap();
    assert_eq!(game.observe().unwrap().consumables.num[0], [l(1.0)]);
}

#[test]
fn every_pack_kind_has_separate_offers_and_only_arcana_spectral_have_a_hand() {
    for (index, kind) in [
        BoosterPackKind::Standard,
        BoosterPackKind::Arcana,
        BoosterPackKind::Celestial,
        BoosterPackKind::Buffoon,
        BoosterPackKind::Spectral,
    ]
    .into_iter()
    .enumerate()
    {
        let mut game = Game::new(515);
        game.enter_shop().unwrap();
        open_pack(&mut game, kind, BoosterPackSize::Mega);
        let observation = game.observe().unwrap();
        all_shapes(&observation);
        let has_hand = matches!(kind, BoosterPackKind::Arcana | BoosterPackKind::Spectral);
        assert_eq!(observation.sections, [false, false, false, true, has_hand]);
        assert_eq!(observation.pack.cat, [index as i32, 2, 2]);
        assert_eq!(observation.pack.counts, [2]);
        assert_eq!(observation.hand.len(), if has_hand { 8 } else { 0 });
        assert_eq!(
            observation.pack_options.len(),
            if matches!(kind, BoosterPackKind::Buffoon | BoosterPackKind::Spectral) {
                4
            } else {
                5
            }
        );
        assert_eq!(observation.shop_items.len(), 0);
        assert_eq!(observation.shop.visible, [false]);
        assert_eq!(observation.blind.visible, [false; 14]);
        for row in 0..observation.pack_options.len() {
            assert_eq!(observation.pack_options.num[row][0], 0.0);
            assert!(!observation.pack_options.visible[row * 20 + 10]);
            if matches!(
                kind,
                BoosterPackKind::Arcana | BoosterPackKind::Spectral | BoosterPackKind::Celestial
            ) {
                assert!(!observation.pack_options.visible[row * 20 + 11]);
            }
        }
        game.apply_action(GameAction::SkipPack).unwrap();
        let after = game.observe().unwrap();
        assert_eq!(after.pack_options.len(), 0);
        assert_eq!(after.pack.visible, [false; 4]);
        assert_eq!(after.hand.len(), 0);
        assert!(after.sections[2]);
    }
}

#[test]
fn pack_consumable_targets_and_options_have_distinct_collections_and_no_stale_prices() {
    let mut game = Game::new(516);
    game.enter_shop().unwrap();
    open_pack(&mut game, BoosterPackKind::Arcana, BoosterPackSize::Mega);
    game.pending_pack.as_mut().unwrap().opening = PackOpening::with_options(
        BoosterPackKind::Arcana,
        BoosterPackSize::Mega,
        vec![
            PackOption::Consumable(Consumable::Tarot(Tarot::Death)),
            PackOption::Consumable(Consumable::Tarot(Tarot::Hermit)),
        ],
    );
    let before = game.observe().unwrap();
    assert_eq!(before.pack_options.cat[0][..3], [2, 14, 1]);
    assert_ne!(before.hand.cat[0], [0; 5]);
    game.apply_action(GameAction::UsePackConsumable {
        index: 0,
        target: ConsumableTarget::hand(vec![0, 3]),
    })
    .unwrap();
    let after = game.observe().unwrap();
    assert_eq!(after.hand.cat[0], before.hand.cat[3]);
    assert_eq!(after.pack_options.cat[0][..3], [2, 10, 1]);
    assert_eq!(after.pack.counts, [1]);
    assert_eq!(before.pack.counts, [2]);
}

#[test]
fn joker_values_parent_indices_follow_inventory_shop_and_pack_order() {
    let mut game = Game::new(517);
    game.state
        .jokers
        .add(Joker::create_joker(JokerKind::Castle));
    game.enter_shop().unwrap();
    game.state.shop.items = vec![ShopItem::Joker {
        joker: Joker::create_joker(JokerKind::Runner),
        price: 5,
    }];
    let before = game.observe().unwrap();
    assert_eq!(
        before
            .joker_values
            .cat
            .iter()
            .map(|row| row[0])
            .collect::<Vec<_>>(),
        [0, 1]
    );
    open_pack(&mut game, BoosterPackKind::Buffoon, BoosterPackSize::Normal);
    game.pending_pack.as_mut().unwrap().opening = PackOpening::with_options(
        BoosterPackKind::Buffoon,
        BoosterPackSize::Normal,
        vec![PackOption::Joker(Joker::create_joker(JokerKind::Runner))],
    );
    let after = game.observe().unwrap();
    assert_eq!(after.joker_values.cat, [[0, 0, 1], [2, 0, 1]]);
}

#[test]
fn empty_and_expanded_collections_have_exact_lengths_without_narrowing() {
    let mut empty = Game::new(518);
    empty.state.deck.remove_cards(52);
    let observation = empty.observe().unwrap();
    all_shapes(&observation);
    assert_eq!(observation.deck_catalogue.len(), 0);
    assert_eq!(observation.run.counts[7], 0);
    let mut game = playing(518);
    let card = game.hand.cards()[0].clone();
    for _ in 0..300 {
        game.hand.add_card(card.clone());
    }
    for _ in 0..300 {
        let mut joker = Joker::create_joker(JokerKind::Juggler);
        joker.set_edition(JokerEdition::Negative);
        game.state.jokers.add(joker);
        game.state.consumables.push(OwnedConsumable::negative(
            Consumable::Tarot(Tarot::Hermit),
            3,
        ));
    }
    let observation = game.observe().unwrap();
    all_shapes(&observation);
    assert_eq!(observation.hand.len(), 308);
    assert_eq!(observation.jokers.len(), 300);
    assert_eq!(observation.consumables.len(), 300);
    assert_eq!(observation.run.counts[2..7], [308, 305, 302, 300, 300]);
    assert_eq!(observation.run.counts[7], 352);
}

#[test]
fn zero_and_large_scores_use_safe_transforms_and_out_of_range_counts_fail() {
    let mut game = playing(519);
    game.state.money = i16::MIN;
    game.state.blind.chips_scored = u64::MAX;
    game.state.blind.score_requirement = 0;
    let before = game.state.rng.get_word_pos();
    let observation = game.observe().unwrap();
    assert!(observation.blind.num[0].is_finite());
    assert_eq!(observation.blind.num[1], 0.0);
    assert!(observation.run.num[0] < 0.0);
    game.state.hand_scores[0].level = i64::MAX as u64 + 1;
    assert_eq!(game.observe(), Err(ObservationError::IntegerOverflow));
    assert_eq!(game.state.rng.get_word_pos(), before);
}

#[test]
fn non_finite_known_card_values_fail_but_concealed_changes_are_not_inspected() {
    let mut game = playing(520);
    game.hand.card_mut(0).unwrap().set_face_down(true);
    let before = game.observe().unwrap();
    game.hand.card_mut(0).unwrap().scoring_values_mut().x_mult = f32::NAN;
    assert_eq!(game.observe().unwrap(), before);
    game.hand.card_mut(0).unwrap().set_face_down(false);
    let rng = game.state.rng.get_word_pos();
    assert_eq!(game.observe(), Err(ObservationError::NonFinite));
    assert_eq!(game.state.rng.get_word_pos(), rng);
}

#[test]
fn repeated_reads_do_not_change_subsequent_seeded_actions_or_old_observations() {
    let mut left = playing(521);
    let mut right = playing(521);
    for step in 0..8 {
        let before = left.observe().unwrap();
        let rng = left.state.rng.get_word_pos();
        for _ in 0..10 {
            assert_eq!(left.observe().unwrap(), before);
        }
        assert_eq!(left.state.rng.get_word_pos(), rng);
        let action = || {
            if step < 4 {
                GameAction::DiscardCards(vec![0, 1])
            } else {
                GameAction::PlayCards(vec![0, 1, 2, 3, 4])
            }
        };
        assert_eq!(left.apply_action(action()), right.apply_action(action()));
        assert_eq!(left.observe().unwrap(), right.observe().unwrap());
        assert_eq!(left.state.rng, right.state.rng);
        if left.decision_point() != DecisionPoint::PlayingHand {
            break;
        }
    }
}

#[test]
fn observation_does_not_open_queued_automatic_packs() {
    let mut game = Game::new(522);
    let opening =
        ShopPack::new(BoosterPackKind::Arcana, BoosterPackSize::Normal, 0).open(&mut game.state);
    game.state.tag_pack_openings.push(opening);
    let before = game.state.rng.get_word_pos();
    assert!(game.observe().unwrap().sections[0]);
    assert_eq!(game.decision_point(), DecisionPoint::BlindSelection);
    assert!(game.pending_pack().is_none());
    assert_eq!(game.state.tag_pack_openings.len(), 1);
    assert_eq!(game.state.rng.get_word_pos(), before);
    game.next_step();
    let after = game.observe().unwrap();
    assert_eq!(after.pack.cat, [1, 0, 0]);
    assert!(after.sections[3]);
    assert!(after.sections[4]);
}

#[test]
fn terminal_observation_preserves_public_inventory_and_clears_inactive_data() {
    let mut game = playing(523);
    game.state
        .add_consumable(Consumable::Tarot(Tarot::Hermit))
        .unwrap();
    game.state.blind.hands_remaining = 1;
    game.apply_action(GameAction::PlayCards(vec![0])).unwrap();
    let observation = game.observe().unwrap();
    all_shapes(&observation);
    assert_eq!(observation.sections, [false; 5]);
    assert_eq!(observation.run.cat[0], 5);
    assert_eq!(observation.run.cat[4], 1);
    assert!(observation.run.flags[0]);
    assert_eq!(observation.action_kinds, [false; 18]);
    assert_eq!(observation.consumables.len(), 1);
    assert_eq!(observation.hand.len(), 0);
    assert_eq!(observation.blind.visible, [false; 14]);
    assert_eq!(observation.pack.visible, [false; 4]);
    assert_eq!(observation.shop.visible, [false]);
}

#[test]
fn ante_eight_success_is_public_history_and_does_not_terminate_endless_play() {
    let mut game = Game::new(524);
    game.state.counters.ante = 8;
    game.state.regenerate_blind_selector();
    game.blind_stage = BlindStage::Boss;
    game.apply_action(GameAction::SelectBlind(game.available_blind()))
        .unwrap();
    game.state.blind.score_requirement = 0;
    let forced = game.state.blind.forced_card_index.unwrap_or(0);
    game.apply_action(GameAction::PlayCards(vec![forced]))
        .unwrap();
    assert_eq!(game.observe().unwrap().run.flags, [false, true, false]);
    game.apply_action(GameAction::LeaveShop).unwrap();
    let after = game.observe().unwrap();
    assert_eq!(after.run.counts[0], 9);
    assert!(after.run.flags[1]);
    assert!(!after.run.flags[0]);
}

#[test]
fn boss_reroll_cost_and_remaining_allowance_and_boss_preview_follow_public_state() {
    let mut game = Game::new(525);
    game.state.money = 100;
    game.state.redeem_voucher(Vouchers::DirectorsCut).unwrap();
    let before = game.observe().unwrap();
    assert_eq!(before.run.num[2], l(10.0));
    assert!(before.run.visible[21]);
    assert_eq!(before.run.extra[2], 1);
    game.apply_action(GameAction::RerollBoss).unwrap();
    let after = game.observe().unwrap();
    assert_eq!(after.run.extra[2], 0);
    assert!(!after.action_kinds[17]);
    assert_eq!(
        after.blind_offers.cat[2][1],
        match game.state.blind_selector.boss().kind {
            BlindKind::Boss(boss) => boss as i32,
            _ => unreachable!(),
        }
    );
    assert_eq!(after.blind_offers.cat[..2], before.blind_offers.cat[..2]);
    game.state.counters.ante = 40;
    game.state.blind_selector.set_boss(BossBlinds::VioletVessel);
    assert!(
        game.observe()
            .unwrap()
            .blind_offers
            .num
            .iter()
            .flatten()
            .all(|value| value.is_finite())
    );
}

#[test]
fn reserved_phase_and_invalid_hand_table_fail_explicitly() {
    let mut game = Game::new(526);
    game.decision_point = DecisionPoint::ConsumableSelection;
    assert_eq!(game.observe(), Err(ObservationError::UnsupportedPhase));
    game.decision_point = DecisionPoint::BlindSelection;
    game.state.hand_scores.pop();
    assert_eq!(
        game.observe(),
        Err(ObservationError::InvalidState("poker-hand table length"))
    );
}

#[test]
fn unsupported_concealed_offers_fail_instead_of_exposing_private_legality() {
    for edition in [JokerEdition::None, JokerEdition::Negative] {
        let mut game = Game::new(528);
        game.enter_shop().unwrap();
        for _ in 0..5 {
            game.state.jokers.add(Joker::create_joker(JokerKind::Joker));
        }
        let mut joker = Joker::create_joker(JokerKind::Joker);
        joker.set_edition(edition);
        joker.set_face_down(true);
        game.state.shop.items = vec![ShopItem::Joker { joker, price: 1 }];
        let rng = game.state.rng.get_word_pos();
        assert_eq!(
            game.observe(),
            Err(ObservationError::InvalidState("concealed offer"))
        );
        assert_eq!(game.state.rng.get_word_pos(), rng);
    }
    let mut game = Game::new(528);
    game.enter_shop().unwrap();
    open_pack(
        &mut game,
        BoosterPackKind::Standard,
        BoosterPackSize::Normal,
    );
    let mut card = Card::new(
        Rank::Two,
        Suit::Hearts,
        Enhancement::None,
        Edition::None,
        Seal::None,
    );
    card.set_face_down(true);
    game.pending_pack.as_mut().unwrap().opening = PackOpening::with_options(
        BoosterPackKind::Standard,
        BoosterPackSize::Normal,
        vec![PackOption::PlayingCard(card)],
    );
    assert_eq!(
        game.observe(),
        Err(ObservationError::InvalidState("concealed offer"))
    );
}
