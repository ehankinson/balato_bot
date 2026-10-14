use super::*;
use crate::core::action::{PermutationCollection, TargetCollection};
use crate::core::enums::{Rank, Suit};
use crate::core::joker_types::{JokerEdition, JokerKind};
use crate::core::pack::{BoosterPackKind, BoosterPackSize, PackOption, ShopPack};
use crate::core::shop::ShopItem;
use rand::{Rng, RngExt};

fn game_at(phase: DecisionPoint) -> Game {
    let mut game = Game::new(920);
    game.state.money = 100;
    match phase {
        DecisionPoint::PlayingHand => {
            game.apply_action(GameAction::SelectBlind(BlindKind::Small))
                .unwrap();
            game.state.blind.score_requirement = u64::MAX;
        }
        DecisionPoint::Shop | DecisionPoint::PackSelection => {
            game.enter_shop().unwrap();
            if phase == DecisionPoint::PackSelection {
                open_pack(&mut game, BoosterPackKind::Standard, BoosterPackSize::Mega);
            }
        }
        DecisionPoint::RunComplete => game.decision_point = DecisionPoint::RunComplete,
        DecisionPoint::BlindSelection => {}
        DecisionPoint::ConsumableSelection => unreachable!(),
    }
    game
}

fn open_pack(game: &mut Game, kind: BoosterPackKind, size: BoosterPackSize) {
    game.state.shop.packs = vec![ShopPack::new(kind, size, 0)];
    game.apply_action(GameAction::BuyPack(0)).unwrap();
}

fn consumable_pack(game: &mut Game, cards: &[Consumable]) {
    open_pack(
        game,
        BoosterPackKind::Arcana,
        if cards.len() > 1 {
            BoosterPackSize::Mega
        } else {
            BoosterPackSize::Normal
        },
    );
    let size = game.pending_pack().unwrap().opening.size();
    game.pending_pack.as_mut().unwrap().opening = PackOpening::with_options(
        BoosterPackKind::Arcana,
        size,
        cards.iter().copied().map(PackOption::Consumable).collect(),
    );
}

/// Compare state and subsequent seeded behavior without cloning the engine or RNG.
fn snapshot(game: &Game) -> String {
    let cards = |cards: &[Card]| {
        cards
            .iter()
            .map(|card| {
                (
                    card.id(),
                    card.is_face_down(),
                    card.is_debuffed(),
                    card.scoring_values(),
                    card.was_played_in_ante(game.state.counters.ante),
                )
            })
            .collect::<Vec<_>>()
    };
    let jokers = game
        .state
        .jokers
        .as_slice()
        .iter()
        .map(|joker| {
            (
                joker.kind(),
                joker.edition(),
                joker.is_debuffed(),
                joker.is_face_down(),
                joker.is_eternal(),
                joker.is_rental(),
                joker.perishable_rounds(),
                joker.sell_value(),
            )
        })
        .collect::<Vec<_>>();
    let pack = game.pending_pack().map(|pending| {
        (
            pending.opening.kind(),
            pending.opening.size(),
            pending.opening.choices(),
            pending
                .opening
                .options()
                .iter()
                .map(|option| match option {
                    PackOption::Joker(joker) => format!("{:?}/{:?}", joker.kind(), joker.edition()),
                    PackOption::Consumable(card) => format!("{card:?}"),
                    PackOption::PlayingCard(card) => format!("{}", card.id()),
                })
                .collect::<Vec<_>>(),
            pending.previous_state,
        )
    });
    format!(
        "{:?}",
        (
            (
                game.decision_point(),
                game.state.money,
                game.state.counters.ante,
                game.state.counters.round,
                game.state.rng.get_word_pos(),
                game.state.blind_selector,
                game.state.counters.boss_rerolls_used
            ),
            (
                cards(game.hand.cards()),
                cards(game.state.deck.cards()),
                cards(game.state.deck.discarded_cards())
            ),
            (
                jokers,
                &game.state.consumables,
                game.state.last_consumable,
                &game.state.vouchers,
                game.state.joker_capacity(),
                game.state.consumable_capacity()
            ),
            (
                game.state.blind.hands_remaining,
                game.state.blind.discards_remaining,
                game.state.blind.chips_scored,
                game.state.blind.forced_card_index,
                &game.state.blind.played_hand_types,
                game.state.counters.tarot_cards_used,
                game.state.counters.planet_cards_used,
                game.state.counters.cards_sold,
                game.state.counters.shop_rerolls
            ),
            pack,
        )
    )
}

fn reject_unchanged(game: &mut Game, action: GameAction) {
    let before = snapshot(game);
    assert!(game.apply_action(action).is_err());
    assert_eq!(snapshot(game), before);
}

#[test]
fn action_codes_and_rng_sequences_preserve_the_control_contract() {
    for (code, kind) in ActionKind::ALL.into_iter().enumerate() {
        assert_eq!(kind as usize, code);
    }
    for seed in [0, 1, 920, u64::MAX] {
        let mut previous = rand::rngs::StdRng::seed_from_u64(seed);
        let mut current = ChaCha12Rng::seed_from_u64(seed);
        for _ in 0..1024 {
            assert_eq!(previous.next_u32(), current.next_u32());
            assert_eq!(previous.next_u64(), current.next_u64());
            assert_eq!(previous.random_range(0..17), current.random_range(0..17));
        }
    }
}

#[test]
fn hand_reorder_uses_old_indices_and_preserves_forced_occurrence() {
    let mut game = game_at(DecisionPoint::PlayingHand);
    game.state.blind.kind = BlindKind::Boss(BossBlinds::CeruleanBell);
    game.state.blind.forced_card_index = Some(0);
    let before = game.hand.cards().iter().map(Card::id).collect::<Vec<_>>();
    let order = [2, 3, 1, 0, 4, 5, 6, 7];
    let rng = game.state.rng.get_word_pos();
    game.apply_action(GameAction::ReorderHand(order.to_vec()))
        .unwrap();
    assert_eq!(
        game.hand.cards().iter().map(Card::id).collect::<Vec<_>>(),
        order.map(|index| before[index])
    );
    assert_eq!(game.state.blind.forced_card_index, Some(3));
    assert_eq!(game.state.rng.get_word_pos(), rng);
    reject_unchanged(&mut game, GameAction::PlayCards(vec![0]));
    game.apply_action(GameAction::PlayCards(vec![3])).unwrap();
    assert_eq!(game.state.blind.hands_remaining, 3);
}

#[test]
fn reordered_jokers_change_blueprint_scoring_through_game_dispatch() {
    for reorder in [false, true] {
        let mut game = game_at(DecisionPoint::PlayingHand);
        for kind in [JokerKind::Blueprint, JokerKind::Joker, JokerKind::Greedy] {
            game.state.jokers.add(Joker::create_joker(kind));
        }
        game.hand = Hand::new();
        game.hand.add_card(Card::new(
            Rank::Two,
            Suit::Clubs,
            Enhancement::None,
            Edition::None,
            Seal::None,
        ));
        if reorder {
            game.apply_action(GameAction::ReorderJokers(vec![2, 1, 0]))
                .unwrap();
        }
        game.apply_action(GameAction::PlayCards(vec![0])).unwrap();
        assert_eq!(game.state.blind.chips_scored, if reorder { 35 } else { 63 });
    }
}

#[test]
fn invalid_permutations_and_inactive_hand_reorders_are_atomic() {
    let mut game = game_at(DecisionPoint::PlayingHand);
    for order in [
        vec![],
        vec![0; 8],
        vec![usize::MAX; 8],
        vec![0, 1, 2, 3, 4, 5, 6],
    ] {
        reject_unchanged(&mut game, GameAction::ReorderHand(order));
    }
    game.state.jokers.add(Joker::create_joker(JokerKind::Joker));
    for order in [vec![], vec![0, 0], vec![usize::MAX]] {
        reject_unchanged(&mut game, GameAction::ReorderJokers(order));
    }
    for phase in [
        DecisionPoint::BlindSelection,
        DecisionPoint::Shop,
        DecisionPoint::PackSelection,
    ] {
        let mut game = game_at(phase);
        reject_unchanged(&mut game, GameAction::ReorderHand(vec![]));
        game.apply_action(GameAction::ReorderJokers(vec![]))
            .unwrap();
        game.state.jokers.add(Joker::create_joker(JokerKind::Joker));
        game.apply_action(GameAction::ReorderJokers(vec![0]))
            .unwrap();
    }
    let mut empty = game_at(DecisionPoint::PlayingHand);
    empty.hand.take_cards();
    empty.apply_action(GameAction::ReorderHand(vec![])).unwrap();
}

#[test]
fn hand_indices_do_not_wrap_at_256() {
    let mut game = game_at(DecisionPoint::PlayingHand);
    game.hand = Hand::new();
    for _ in 0..300 {
        game.hand.add_card(Card::new(
            Rank::Two,
            Suit::Clubs,
            Enhancement::None,
            Edition::None,
            Seal::None,
        ));
    }
    assert_eq!(game.hand().hand_size(), 300);
    reject_unchanged(&mut game, GameAction::DiscardCards(vec![300]));
    game.apply_action(GameAction::DiscardCards(vec![299]))
        .unwrap();
    assert_eq!(game.hand().hand_size(), 299);
    game.apply_action(GameAction::PlayCards(vec![298])).unwrap();
    assert_eq!(game.state.blind.hands_remaining, 3);
}

#[test]
fn inventory_management_is_available_in_every_nonterminal_phase() {
    for phase in [
        DecisionPoint::BlindSelection,
        DecisionPoint::PlayingHand,
        DecisionPoint::Shop,
        DecisionPoint::PackSelection,
    ] {
        let mut game = game_at(phase);
        game.state
            .add_consumable(Consumable::Tarot(Tarot::Hermit))
            .unwrap();
        game.apply_action(GameAction::UseConsumable {
            index: 0,
            target: ConsumableTarget::default(),
        })
        .unwrap();
        assert!(game.state.consumables.is_empty());
        assert_eq!(game.decision_point(), phase);
        game.state
            .add_consumable(Consumable::Planet(Planet::Pluto))
            .unwrap();
        game.state.jokers.add(Joker::create_joker(JokerKind::Joker));
        game.apply_action(GameAction::SellConsumable(0)).unwrap();
        game.apply_action(GameAction::SellJoker(0)).unwrap();
        assert!(game.state.consumables.is_empty());
        assert!(game.state.jokers.as_slice().is_empty());
    }
}

#[test]
fn stale_shop_hands_cannot_be_targeted_or_used_by_automatic_hand_effects() {
    let mut game = game_at(DecisionPoint::PlayingHand);
    game.state.blind.score_requirement = 0;
    game.apply_action(GameAction::PlayCards(vec![0])).unwrap();
    assert_eq!(game.decision_point(), DecisionPoint::Shop);
    assert!(!game.hand.cards().is_empty());
    assert!(game.active_hand().is_none());
    for card in [
        Consumable::Tarot(Tarot::Magician),
        Consumable::Spectral(Spectral::Sigil),
    ] {
        game.state.consumables.clear();
        game.state.add_consumable(card).unwrap();
        reject_unchanged(
            &mut game,
            GameAction::UseConsumable {
                index: 0,
                target: ConsumableTarget::hand(if card.targets().maximum > 0 {
                    vec![0]
                } else {
                    vec![]
                }),
            },
        );
        assert!(
            !game
                .action_constraints(ActionKind::UseConsumable, Some(0))
                .unwrap()
                .available
        );
    }
}

#[test]
fn arcana_pack_deals_once_and_returns_mutated_generated_cards_on_close() {
    let mut game = game_at(DecisionPoint::Shop);
    consumable_pack(
        &mut game,
        &[
            Consumable::Tarot(Tarot::Death),
            Consumable::Spectral(Spectral::Cryptid),
        ],
    );
    assert_eq!(game.hand.cards().len(), 8);
    assert_eq!(game.state.deck.cards().len(), 44);
    let copied = game.hand.cards()[3].id();
    let rng = game.state.rng.get_word_pos();
    game.apply_action(GameAction::UsePackConsumable {
        index: 0,
        target: ConsumableTarget::hand(vec![0, 3]),
    })
    .unwrap();
    assert_eq!(game.hand.cards()[0].id(), copied);
    assert_eq!(game.state.rng.get_word_pos(), rng);
    assert_eq!(game.pending_pack().unwrap().opening.choices(), 1);
    game.apply_action(GameAction::UsePackConsumable {
        index: 0,
        target: ConsumableTarget::hand(vec![0]),
    })
    .unwrap();
    assert_eq!(game.decision_point(), DecisionPoint::Shop);
    assert!(game.hand.cards().is_empty());
    assert!(game.pending_pack().is_none());
    assert_eq!(game.state.deck.cards().len(), 54);
    assert!(game.state.deck.discarded_cards().is_empty());
    assert!(
        game.state
            .deck
            .cards()
            .iter()
            .all(|card| !card.is_face_down() && !card.is_debuffed())
    );
    assert!(game.state.consumables.is_empty());
    assert_eq!(game.state.counters.tarot_cards_used, 1);
}

#[test]
fn every_pack_consumable_uses_the_game_command_with_valid_targets() {
    let cards = Tarot::ALL
        .into_iter()
        .map(Consumable::Tarot)
        .chain(Planet::ALL.into_iter().map(Consumable::Planet))
        .chain(Spectral::ALL.into_iter().map(Consumable::Spectral));
    for card in cards {
        let mut game = game_at(DecisionPoint::Shop);
        game.state.jokers.add(Joker::create_joker(JokerKind::Joker));
        game.state.last_consumable =
            Some(OwnedConsumable::new(Consumable::Tarot(Tarot::Hermit), 3));
        consumable_pack(&mut game, &[card]);
        let targets = (0..card.targets().minimum).collect::<Vec<_>>();
        assert!(
            game.action_constraints(ActionKind::UsePackConsumable, Some(0))
                .unwrap()
                .source_mask[0],
            "{card:?}"
        );
        let result = game.apply_action(GameAction::UsePackConsumable {
            index: 0,
            target: ConsumableTarget::hand(targets),
        });
        assert_eq!(result, Ok(DecisionPoint::Shop), "{card:?}");
        assert!(game.pending_pack().is_none());
        assert!(game.hand.cards().is_empty());
    }
}

#[test]
fn pack_skip_and_failure_preserve_the_remaining_choice_until_commit() {
    let mut game = game_at(DecisionPoint::Shop);
    game.state
        .jokers
        .add(Joker::create_joker(JokerKind::RedCard));
    consumable_pack(
        &mut game,
        &[
            Consumable::Tarot(Tarot::Death),
            Consumable::Tarot(Tarot::Hermit),
        ],
    );
    for indices in [
        vec![],
        vec![0],
        vec![0, 0],
        vec![0, usize::MAX],
        vec![0, 1, 2],
    ] {
        reject_unchanged(
            &mut game,
            GameAction::UsePackConsumable {
                index: 0,
                target: ConsumableTarget::hand(indices),
            },
        );
    }
    reject_unchanged(&mut game, GameAction::SelectPackOption(0));
    reject_unchanged(
        &mut game,
        GameAction::UsePackConsumable {
            index: usize::MAX,
            target: ConsumableTarget::default(),
        },
    );
    game.apply_action(GameAction::UsePackConsumable {
        index: 0,
        target: ConsumableTarget::hand(vec![1, 0]),
    })
    .unwrap();
    game.apply_action(GameAction::SkipPack).unwrap();
    assert_eq!(game.decision_point(), DecisionPoint::Shop);
    reject_unchanged(&mut game, GameAction::SkipPack);
    game.apply_action(GameAction::LeaveShop).unwrap();
    game.apply_action(GameAction::SelectBlind(BlindKind::Small))
        .unwrap();
    game.state.blind.score_requirement = u64::MAX;
    game.hand = Hand::new();
    game.hand.add_card(Card::new(
        Rank::Two,
        Suit::Clubs,
        Enhancement::None,
        Edition::None,
        Seal::None,
    ));
    game.apply_action(GameAction::PlayCards(vec![0])).unwrap();
    assert_eq!(
        game.state.blind.chips_scored, 28,
        "Red Card skip grows its mult exactly once"
    );
}

#[test]
fn non_hand_packs_do_not_deal_or_accept_hand_uses() {
    for kind in [
        BoosterPackKind::Celestial,
        BoosterPackKind::Standard,
        BoosterPackKind::Buffoon,
    ] {
        let mut game = game_at(DecisionPoint::Shop);
        let draw = game.state.deck.cards().len();
        open_pack(&mut game, kind, BoosterPackSize::Normal);
        assert!(game.active_hand().is_none());
        assert!(game.hand.cards().is_empty());
        assert_eq!(game.state.deck.cards().len(), draw);
        game.state
            .add_consumable(Consumable::Tarot(Tarot::Death))
            .unwrap();
        reject_unchanged(
            &mut game,
            GameAction::UseConsumable {
                index: 0,
                target: ConsumableTarget::hand(vec![0, 1]),
            },
        );
        if kind != BoosterPackKind::Celestial {
            reject_unchanged(
                &mut game,
                GameAction::UsePackConsumable {
                    index: 0,
                    target: ConsumableTarget::default(),
                },
            );
        }
        game.apply_action(GameAction::SkipPack).unwrap();
    }
}

#[test]
fn pack_creators_have_no_released_slot_but_normal_owned_sources_do() {
    for negative in [false, true] {
        let mut game = game_at(DecisionPoint::BlindSelection);
        let card = Consumable::Tarot(Tarot::Emperor);
        game.state.consumables = vec![
            if negative {
                OwnedConsumable::negative(card, 3)
            } else {
                OwnedConsumable::new(card, 3)
            },
            OwnedConsumable::new(Consumable::Planet(Planet::Pluto), 3),
        ];
        if negative {
            game.state
                .consumables
                .push(OwnedConsumable::new(Consumable::Planet(Planet::Mercury), 3));
        }
        if negative {
            reject_unchanged(
                &mut game,
                GameAction::UseConsumable {
                    index: 0,
                    target: ConsumableTarget::default(),
                },
            );
        } else {
            game.apply_action(GameAction::UseConsumable {
                index: 0,
                target: ConsumableTarget::default(),
            })
            .unwrap();
            assert_eq!(game.state.consumables.len(), 2);
        }
    }
    let mut game = game_at(DecisionPoint::Shop);
    game.state
        .add_consumable(Consumable::Planet(Planet::Pluto))
        .unwrap();
    game.state
        .add_consumable(Consumable::Planet(Planet::Mercury))
        .unwrap();
    consumable_pack(&mut game, &[Consumable::Tarot(Tarot::Emperor)]);
    reject_unchanged(
        &mut game,
        GameAction::UsePackConsumable {
            index: 0,
            target: ConsumableTarget::default(),
        },
    );
    game.apply_action(GameAction::SellConsumable(0)).unwrap();
    game.apply_action(GameAction::UsePackConsumable {
        index: 0,
        target: ConsumableTarget::default(),
    })
    .unwrap();
    assert_eq!(game.state.consumables.len(), 2);
}

#[test]
fn fool_can_copy_a_negative_consumable_at_full_capacity() {
    let mut game = game_at(DecisionPoint::Shop);
    game.state.last_consumable = Some(OwnedConsumable::negative(
        Consumable::Planet(Planet::Pluto),
        3,
    ));
    game.state
        .add_consumable(Consumable::Planet(Planet::Pluto))
        .unwrap();
    game.state
        .add_consumable(Consumable::Planet(Planet::Mercury))
        .unwrap();
    consumable_pack(&mut game, &[Consumable::Tarot(Tarot::Fool)]);
    game.apply_action(GameAction::UsePackConsumable {
        index: 0,
        target: ConsumableTarget::default(),
    })
    .unwrap();
    assert_eq!(game.state.consumables.len(), 3);
    assert!(game.state.consumables[2].negative);
    assert_eq!(game.state.consumable_capacity(), 3);
}

#[test]
fn negative_joker_acquisition_and_sale_use_the_same_effective_capacity() {
    for in_pack in [false, true] {
        let mut game = game_at(DecisionPoint::Shop);
        for _ in 0..5 {
            game.state.jokers.add(Joker::create_joker(JokerKind::Joker));
        }
        let mut negative = Joker::create_joker(JokerKind::Greedy);
        negative.set_edition(JokerEdition::Negative);
        if in_pack {
            open_pack(&mut game, BoosterPackKind::Buffoon, BoosterPackSize::Normal);
            game.pending_pack.as_mut().unwrap().opening = PackOpening::with_options(
                BoosterPackKind::Buffoon,
                BoosterPackSize::Normal,
                vec![PackOption::Joker(negative)],
            );
            game.apply_action(GameAction::SelectPackOption(0)).unwrap();
        } else {
            game.state.shop.items = vec![ShopItem::Joker {
                joker: negative,
                price: 1,
            }];
            game.apply_action(GameAction::BuyShopItem(0)).unwrap();
        }
        assert_eq!(game.state.jokers.as_slice().len(), 6);
        assert_eq!(game.state.joker_capacity(), 6);
        game.apply_action(GameAction::SellJoker(5)).unwrap();
        assert_eq!(game.state.joker_capacity(), 5);
        game.state.shop.items = vec![ShopItem::Joker {
            joker: Joker::create_joker(JokerKind::Greedy),
            price: 1,
        }];
        reject_unchanged(&mut game, GameAction::BuyShopItem(0));
    }
}

#[test]
fn boss_reroll_cost_allowance_and_next_ante_reset_are_shared_rules() {
    let mut game = game_at(DecisionPoint::BlindSelection);
    reject_unchanged(&mut game, GameAction::RerollBoss);
    game.state.redeem_voucher(Vouchers::DirectorsCut).unwrap();
    game.state.money = 9;
    assert!(!game.action_kinds()[ActionKind::RerollBoss as usize]);
    reject_unchanged(&mut game, GameAction::RerollBoss);
    game.state.money = 10;
    let selector = game.state.blind_selector;
    assert!(game.action_kinds()[ActionKind::RerollBoss as usize]);
    game.apply_action(GameAction::RerollBoss).unwrap();
    assert_eq!(game.state.money, 0);
    assert_eq!(game.state.blind_selector.small(), selector.small());
    assert_eq!(game.state.blind_selector.big(), selector.big());
    assert_eq!(game.state.counters.boss_rerolls_used, 1);
    game.state.money = 100;
    reject_unchanged(&mut game, GameAction::RerollBoss);
    // Complete the existing Small/Big/Boss cycle through commands to reset allowance.
    for _ in 0..3 {
        game.apply_action(GameAction::SelectBlind(game.available_blind()))
            .unwrap();
        game.state.blind.score_requirement = 0;
        let forced = game.state.blind.forced_card_index.unwrap_or(0);
        game.apply_action(GameAction::PlayCards(vec![forced]))
            .unwrap();
        reject_unchanged(&mut game, GameAction::RerollBoss);
        game.apply_action(GameAction::LeaveShop).unwrap();
    }
    assert_eq!(game.state.counters.ante, 2);
    assert_eq!(game.state.counters.boss_rerolls_used, 0);
    game.apply_action(GameAction::RerollBoss).unwrap();
    game.state.redeem_voucher(Vouchers::Retcon).unwrap();
    for _ in 0..3 {
        game.apply_action(GameAction::RerollBoss).unwrap();
    }
    assert_eq!(game.state.counters.boss_rerolls_used, 4);
}

#[test]
fn invalid_card_targets_and_surplus_consumable_targets_do_not_mutate() {
    let mut game = game_at(DecisionPoint::PlayingHand);
    for targets in [vec![], vec![0, 0], vec![usize::MAX], vec![0, 1, 2, 3, 4, 5]] {
        reject_unchanged(&mut game, GameAction::PlayCards(targets.clone()));
        reject_unchanged(&mut game, GameAction::DiscardCards(targets));
    }
    game.state
        .add_consumable(Consumable::Tarot(Tarot::Hermit))
        .unwrap();
    reject_unchanged(
        &mut game,
        GameAction::UseConsumable {
            index: 0,
            target: ConsumableTarget::hand(vec![0]),
        },
    );
    reject_unchanged(
        &mut game,
        GameAction::UseConsumable {
            index: usize::MAX,
            target: ConsumableTarget::default(),
        },
    );
    game.state.blind.discards_remaining = 0;
    reject_unchanged(&mut game, GameAction::DiscardCards(vec![0]));
}

#[test]
fn large_shop_prices_do_not_wrap_into_affordable_negative_prices() {
    let mut game = game_at(DecisionPoint::Shop);
    game.state.shop.items = vec![ShopItem::Consumable {
        consumable: Consumable::Planet(Planet::Pluto),
        price: u16::MAX,
    }];
    game.state.shop.reroll_cost = u16::MAX;
    game.state.shop.packs = vec![ShopPack::new(
        BoosterPackKind::Standard,
        BoosterPackSize::Normal,
        u16::MAX,
    )];
    game.state.shop.voucher = Some(crate::core::shop::ShopVoucher {
        voucher: Vouchers::Overstock,
        price: u16::MAX,
    });
    for action in [
        GameAction::BuyShopItem(0),
        GameAction::RerollShop,
        GameAction::BuyPack(0),
        GameAction::BuyVoucher(0),
    ] {
        reject_unchanged(&mut game, action);
    }
    for kind in [
        ActionKind::BuyShopItem,
        ActionKind::BuyVoucher,
        ActionKind::RerollShop,
        ActionKind::BuyPack,
    ] {
        assert!(!game.action_kinds()[kind as usize]);
    }
}

#[test]
fn constraint_reads_are_seeded_read_only_and_report_directed_death_targets() {
    let mut game = game_at(DecisionPoint::PlayingHand);
    game.state
        .add_consumable(Consumable::Tarot(Tarot::Death))
        .unwrap();
    let before = snapshot(&game);
    let constraints = game
        .action_constraints(ActionKind::UseConsumable, Some(0))
        .unwrap();
    assert!(constraints.ordered);
    assert_eq!(constraints.target_collection, TargetCollection::Hand);
    assert_eq!(
        constraints.count_mask.iter().position(|valid| *valid),
        Some(2)
    );
    assert!(!constraints.can_submit);
    let prefix = game
        .selection_constraints(ActionKind::UseConsumable, Some(0), &[3])
        .unwrap();
    assert!(!prefix.target_mask[3]);
    assert!(prefix.target_mask[0]);
    assert!(!prefix.can_submit);
    let complete = game
        .selection_constraints(ActionKind::UseConsumable, Some(0), &[3, 0])
        .unwrap();
    assert!(complete.can_submit);
    assert!(complete.target_mask.iter().all(|valid| !valid));
    for prefix in [vec![0, 0], vec![99], vec![0, 1, 2]] {
        assert_eq!(
            game.selection_constraints(ActionKind::UseConsumable, Some(0), &prefix),
            Err(ActionError::InvalidAction)
        );
    }
    assert_eq!(
        game.action_constraints(ActionKind::UseConsumable, Some(0))
            .unwrap(),
        constraints
    );
    assert_eq!(snapshot(&game), before);
}

#[test]
fn forced_play_constraints_allow_discards_and_require_the_final_play_slot() {
    let mut game = game_at(DecisionPoint::PlayingHand);
    game.state.blind.kind = BlindKind::Boss(BossBlinds::CeruleanBell);
    game.state.blind.forced_card_index = Some(7);
    let constraints = game
        .selection_constraints(ActionKind::PlayCards, None, &[0, 1, 2, 3])
        .unwrap();
    assert!(!constraints.can_submit);
    assert_eq!(
        constraints.target_mask,
        vec![false, false, false, false, false, false, false, true]
    );
    let discard = game
        .selection_constraints(ActionKind::DiscardCards, None, &[0])
        .unwrap();
    assert!(discard.can_submit);
    assert!(!discard.required_targets.iter().any(|required| *required));
    assert!(
        game.selection_constraints(ActionKind::PlayCards, None, &[0, 1, 2, 3, 7])
            .unwrap()
            .can_submit
    );
    assert!(
        game.selection_constraints(ActionKind::PlayCards, None, &[0, 1, 2, 3, 4])
            .is_err()
    );
    for boss in [BossBlinds::Psychich, BossBlinds::Eye, BossBlinds::Mouth] {
        game.state.blind.kind = BlindKind::Boss(boss);
        assert!(
            game.selection_constraints(ActionKind::PlayCards, None, &[0])
                .unwrap()
                .can_submit
        );
        assert!(
            game.validate_action(&GameAction::PlayCards(vec![0]))
                .is_ok()
        );
    }
}

#[test]
fn destroying_a_card_remaps_or_reselects_bells_forced_occurrence() {
    for destroy_forced in [false, true] {
        let mut game = game_at(DecisionPoint::PlayingHand);
        game.state.blind.kind = BlindKind::Boss(BossBlinds::CeruleanBell);
        game.state.blind.forced_card_index = Some(3);
        let forced_id = game.hand.cards()[3].id();
        game.state
            .add_consumable(Consumable::Tarot(Tarot::HangedMan))
            .unwrap();
        game.apply_action(GameAction::UseConsumable {
            index: 0,
            target: ConsumableTarget::hand(vec![if destroy_forced { 3 } else { 0 }]),
        })
        .unwrap();
        let forced = game
            .state
            .blind
            .forced_card_index
            .expect("Bell still forces a surviving occurrence");
        assert!(forced < game.hand.cards().len());
        if !destroy_forced {
            assert_eq!(forced, 2);
            assert_eq!(game.hand.cards()[forced].id(), forced_id);
        }
        let other = (forced + 1) % game.hand.cards().len();
        reject_unchanged(&mut game, GameAction::PlayCards(vec![other]));
        game.apply_action(GameAction::PlayCards(vec![forced]))
            .unwrap();
    }
}

#[test]
fn targeted_mutation_refreshes_visible_boss_debuffs_before_the_next_decision() {
    let mut game = game_at(DecisionPoint::PlayingHand);
    game.state.blind.kind = BlindKind::Boss(BossBlinds::Club);
    game.hand.card_mut(0).unwrap().set_suit(Suit::Clubs);
    game.hand.card_mut(0).unwrap().set_debuffed(true);
    game.state
        .add_consumable(Consumable::Tarot(Tarot::Sun))
        .unwrap();
    game.apply_action(GameAction::UseConsumable {
        index: 0,
        target: ConsumableTarget::hand(vec![0]),
    })
    .unwrap();
    assert_eq!(game.hand.cards()[0].suit(), Suit::Hearts);
    assert!(!game.hand.cards()[0].is_debuffed());
}

#[test]
fn temporary_hand_pack_reorder_is_allowed_and_skip_preserves_destruction() {
    let mut game = game_at(DecisionPoint::Shop);
    consumable_pack(
        &mut game,
        &[
            Consumable::Tarot(Tarot::HangedMan),
            Consumable::Tarot(Tarot::Hermit),
        ],
    );
    let before = game.hand.cards().iter().map(Card::id).collect::<Vec<_>>();
    game.apply_action(GameAction::ReorderHand((0..8).rev().collect()))
        .unwrap();
    assert_eq!(
        game.hand.cards().iter().map(Card::id).collect::<Vec<_>>(),
        before.into_iter().rev().collect::<Vec<_>>()
    );
    game.apply_action(GameAction::UsePackConsumable {
        index: 0,
        target: ConsumableTarget::hand(vec![0, 1]),
    })
    .unwrap();
    assert_eq!(game.hand.cards().len(), 6);
    game.apply_action(GameAction::SkipPack).unwrap();
    assert_eq!(game.state.deck.cards().len(), 50);
    assert!(game.hand.cards().is_empty());
    reject_unchanged(&mut game, GameAction::ReorderHand(vec![]));
}

#[test]
fn queued_tag_packs_return_cards_between_openings_and_resume_the_next_blind() {
    let mut game = game_at(DecisionPoint::BlindSelection);
    let mut tags = std::mem::take(&mut game.state.tags);
    tags.gain(SkipTag::Double, &mut game.state);
    game.state.tags = tags;
    game.state.blind_selector.set_small_skip_tag(SkipTag::Charm);
    game.apply_action(GameAction::SkipBlind(BlindKind::Small))
        .unwrap();
    assert_eq!(
        game.pending_pack().unwrap().opening.kind(),
        BoosterPackKind::Arcana
    );
    assert_eq!(game.state.tag_pack_openings.len(), 1);
    assert_eq!(game.hand.cards().len(), 8);
    assert_eq!(game.state.deck.cards().len(), 44);
    game.apply_action(GameAction::SkipPack).unwrap();
    assert!(game.state.tag_pack_openings.is_empty());
    assert_eq!(game.decision_point(), DecisionPoint::PackSelection);
    assert_eq!(game.hand.cards().len(), 8);
    assert_eq!(game.state.deck.cards().len(), 44);
    game.apply_action(GameAction::SkipPack).unwrap();
    assert_eq!(game.decision_point(), DecisionPoint::BlindSelection);
    assert_eq!(game.available_blind(), BlindKind::Big);
    assert!(game.hand.cards().is_empty());
    assert_eq!(game.state.deck.cards().len(), 52);
}

#[test]
fn known_source_masks_agree_with_command_validation_for_shop_and_inventory() {
    let mut game = game_at(DecisionPoint::Shop);
    game.state.money = 4;
    game.state.shop.items = vec![
        ShopItem::Consumable {
            consumable: Consumable::Planet(Planet::Pluto),
            price: 4,
        },
        ShopItem::Consumable {
            consumable: Consumable::Planet(Planet::Mercury),
            price: 5,
        },
    ];
    game.state.jokers.add(Joker::create_joker(JokerKind::Joker));
    let mut eternal = Joker::create_joker(JokerKind::Greedy);
    eternal.set_eternal();
    game.state.jokers.add(eternal);
    for kind in [
        ActionKind::BuyShopItem,
        ActionKind::SellJoker,
        ActionKind::BuyVoucher,
        ActionKind::BuyPack,
    ] {
        let constraints = game.action_constraints(kind, None).unwrap();
        assert!(constraints.source_known.iter().all(|known| *known));
        for (index, feasible) in constraints.source_mask.into_iter().enumerate() {
            let command = match kind {
                ActionKind::BuyShopItem => GameAction::BuyShopItem(index),
                ActionKind::SellJoker => GameAction::SellJoker(index),
                ActionKind::BuyVoucher => GameAction::BuyVoucher(index),
                _ => GameAction::BuyPack(index),
            };
            assert_eq!(feasible, game.validate_action(&command).is_ok());
        }
    }
    for kind in [ActionKind::RerollShop, ActionKind::LeaveShop] {
        let command = if kind == ActionKind::RerollShop {
            GameAction::RerollShop
        } else {
            GameAction::LeaveShop
        };
        assert_eq!(
            game.action_constraints(kind, None).unwrap().can_submit,
            game.validate_action(&command).is_ok()
        );
    }
    reject_unchanged(&mut game, GameAction::BuyShopItem(usize::MAX));
    reject_unchanged(&mut game, GameAction::BuyVoucher(usize::MAX));
    reject_unchanged(&mut game, GameAction::BuyPack(usize::MAX));
    reject_unchanged(&mut game, GameAction::SellConsumable(usize::MAX));
}

#[test]
fn pack_generated_cards_update_hologram_through_the_existing_deck_event() {
    let mut game = game_at(DecisionPoint::Shop);
    game.state
        .jokers
        .add(Joker::create_joker(JokerKind::Hologram));
    consumable_pack(&mut game, &[Consumable::Spectral(Spectral::Cryptid)]);
    game.apply_action(GameAction::UsePackConsumable {
        index: 0,
        target: ConsumableTarget::hand(vec![0]),
    })
    .unwrap();
    game.apply_action(GameAction::LeaveShop).unwrap();
    game.apply_action(GameAction::SelectBlind(BlindKind::Small))
        .unwrap();
    game.state.blind.score_requirement = u64::MAX;
    game.hand = Hand::new();
    game.hand.add_card(Card::new(
        Rank::Two,
        Suit::Clubs,
        Enhancement::None,
        Edition::None,
        Seal::None,
    ));
    game.apply_action(GameAction::PlayCards(vec![0])).unwrap();
    assert_eq!(
        game.state.blind.chips_scored, 10,
        "Hologram gained 0.25 xMult for each generated card"
    );
}

#[test]
fn owned_and_pack_consumable_sources_are_distinct_at_the_same_index() {
    let mut game = game_at(DecisionPoint::Shop);
    game.state
        .add_consumable(Consumable::Tarot(Tarot::Hermit))
        .unwrap();
    consumable_pack(
        &mut game,
        &[
            Consumable::Tarot(Tarot::Death),
            Consumable::Tarot(Tarot::Hermit),
        ],
    );
    let hand = game.hand.cards().iter().map(Card::id).collect::<Vec<_>>();
    game.apply_action(GameAction::UseConsumable {
        index: 0,
        target: ConsumableTarget::default(),
    })
    .unwrap();
    assert!(game.state.consumables.is_empty());
    assert_eq!(game.pending_pack().unwrap().opening.choices(), 2);
    assert_eq!(
        game.hand.cards().iter().map(Card::id).collect::<Vec<_>>(),
        hand
    );
    game.apply_action(GameAction::UsePackConsumable {
        index: 0,
        target: ConsumableTarget::hand(vec![0, 1]),
    })
    .unwrap();
    assert_eq!(game.hand.cards()[0].id(), hand[1]);
    assert_eq!(game.pending_pack().unwrap().opening.choices(), 1);
}

#[test]
fn wrong_phase_commands_fail_before_any_automatic_or_rule_work() {
    let mut selection = game_at(DecisionPoint::BlindSelection);
    for command in [
        GameAction::PlayCards(vec![0]),
        GameAction::DiscardCards(vec![0]),
        GameAction::BuyShopItem(0),
        GameAction::BuyVoucher(0),
        GameAction::RerollShop,
        GameAction::BuyPack(0),
        GameAction::LeaveShop,
        GameAction::SkipPack,
        GameAction::SelectPackOption(0),
        GameAction::UsePackConsumable {
            index: 0,
            target: ConsumableTarget::default(),
        },
    ] {
        reject_unchanged(&mut selection, command);
    }
    let mut playing = game_at(DecisionPoint::PlayingHand);
    for command in [
        GameAction::SelectBlind(BlindKind::Small),
        GameAction::SkipBlind(BlindKind::Small),
        GameAction::RerollBoss,
        GameAction::BuyShopItem(0),
        GameAction::LeaveShop,
    ] {
        reject_unchanged(&mut playing, command);
    }
    let mut packed = game_at(DecisionPoint::PackSelection);
    for kind in [
        ActionKind::BuyShopItem,
        ActionKind::BuyVoucher,
        ActionKind::BuyPack,
    ] {
        assert!(
            packed
                .action_constraints(kind, None)
                .unwrap()
                .source_mask
                .is_empty()
        );
    }
    for command in [
        GameAction::BuyPack(0),
        GameAction::BuyShopItem(0),
        GameAction::BuyVoucher(0),
        GameAction::RerollShop,
        GameAction::LeaveShop,
    ] {
        reject_unchanged(&mut packed, command);
    }
}

#[test]
fn reorder_prefixes_require_each_existing_slot_once() {
    let mut game = game_at(DecisionPoint::Shop);
    for _ in 0..3 {
        game.state.jokers.add(Joker::create_joker(JokerKind::Joker));
    }
    let prefix = game
        .selection_constraints(ActionKind::ReorderJokers, None, &[2, 0])
        .unwrap();
    assert_eq!(prefix.permutation_collection, PermutationCollection::Jokers);
    assert_eq!(prefix.target_mask, vec![false, true, false]);
    assert!(!prefix.can_submit);
    assert!(
        game.selection_constraints(ActionKind::ReorderJokers, None, &[2, 0, 1])
            .unwrap()
            .can_submit
    );
    assert!(
        game.selection_constraints(ActionKind::ReorderJokers, None, &[0, 0])
            .is_err()
    );
}

#[test]
fn hidden_joker_constraints_do_not_reveal_eternal_status_or_slot_identity() {
    let mut game = game_at(DecisionPoint::PlayingHand);
    for _ in 0..5 {
        let mut joker = Joker::create_joker(JokerKind::Joker);
        joker.set_face_down(true);
        game.state.jokers.add(joker);
    }
    game.state
        .add_consumable(Consumable::Spectral(Spectral::Ankh))
        .unwrap();
    let sale = game
        .action_constraints(ActionKind::SellJoker, None)
        .unwrap();
    let ankh = game
        .action_constraints(ActionKind::UseConsumable, Some(0))
        .unwrap();
    for joker in game.state.jokers.as_mut_slice() {
        joker.set_eternal();
    }
    assert_eq!(
        game.action_constraints(ActionKind::SellJoker, None)
            .unwrap(),
        sale
    );
    assert_eq!(
        game.action_constraints(ActionKind::UseConsumable, Some(0))
            .unwrap(),
        ankh
    );
    assert_eq!(sale.source_known, vec![false; 5]);
    assert_eq!(sale.source_mask, vec![true; 5]);
    reject_unchanged(&mut game, GameAction::SellJoker(0));
    reject_unchanged(
        &mut game,
        GameAction::UseConsumable {
            index: 0,
            target: ConsumableTarget::default(),
        },
    );
}

#[test]
fn failed_random_ankh_choice_rewinds_rng_for_owned_and_pack_sources() {
    for pack_source in [false, true] {
        let mut observed_failure = false;
        let mut observed_success = false;
        for seed in 0..64 {
            let mut game = Game::new(seed);
            game.enter_shop().unwrap();
            for index in 0..5 {
                let mut joker = Joker::create_joker(JokerKind::Joker);
                if index < 4 {
                    joker.set_eternal();
                }
                game.state.jokers.add(joker);
            }
            let card = Consumable::Spectral(Spectral::Ankh);
            if pack_source {
                consumable_pack(&mut game, &[card]);
            } else {
                game.state.add_consumable(card).unwrap();
            }
            let before = snapshot(&game);
            let action = if pack_source {
                GameAction::UsePackConsumable {
                    index: 0,
                    target: ConsumableTarget::default(),
                }
            } else {
                GameAction::UseConsumable {
                    index: 0,
                    target: ConsumableTarget::default(),
                }
            };
            if game.apply_action(action).is_err() {
                assert_eq!(snapshot(&game), before);
                observed_failure = true;
            } else {
                observed_success = true;
            }
        }
        assert!(observed_failure && observed_success);
    }
}

#[test]
fn terminal_state_rejects_every_action_and_exposes_no_legal_kind() {
    let mut game = game_at(DecisionPoint::RunComplete);
    assert_eq!(game.action_kinds(), [false; 18]);
    for action in [
        GameAction::SelectBlind(BlindKind::Small),
        GameAction::SkipBlind(BlindKind::Small),
        GameAction::PlayCards(vec![0]),
        GameAction::DiscardCards(vec![0]),
        GameAction::BuyShopItem(0),
        GameAction::BuyVoucher(0),
        GameAction::RerollShop,
        GameAction::BuyPack(0),
        GameAction::SelectPackOption(0),
        GameAction::UseConsumable {
            index: 0,
            target: ConsumableTarget::default(),
        },
        GameAction::SellJoker(0),
        GameAction::SellConsumable(0),
        GameAction::LeaveShop,
        GameAction::SkipPack,
        GameAction::UsePackConsumable {
            index: 0,
            target: ConsumableTarget::default(),
        },
        GameAction::ReorderJokers(vec![]),
        GameAction::ReorderHand(vec![]),
        GameAction::RerollBoss,
    ] {
        reject_unchanged(&mut game, action);
    }
    assert_eq!(game.next_step(), DecisionPoint::RunComplete);
}
