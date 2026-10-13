use super::*;

use crate::core::blind::{BlindKind, BlindSkipError};
use crate::core::enums::{BossBlinds, SkipTag};
use crate::core::pack::{BoosterPackKind, BoosterPackSize, ShopPack};
use crate::core::shop::{ShopItem, ShopState};
use crate::core::tag::TagState;

fn apply_tag(state: &mut GameState, tag: SkipTag) {
    let mut tags = std::mem::take(&mut state.tags);
    tags.gain(tag, state);
    state.tags = tags;
}

#[test]
fn double_tag_copies_every_other_tag() {
    for tag in SkipTag::ALL {
        if tag == SkipTag::Double {
            continue;
        }
        let mut state = GameState::new(tag as u64 + 100);
        apply_tag(&mut state, SkipTag::Double);
        apply_tag(&mut state, tag);

        let copies = state
            .tags
            .held()
            .iter()
            .filter(|held| **held == tag)
            .count();
        assert_eq!(copies, 2, "Double should copy {tag:?}");
    }
}

#[test]
fn economy_tag_doubles_positive_money_and_caps_the_result() {
    let mut state = GameState::new(1);
    state.money = 30;

    apply_tag(&mut state, SkipTag::Economy);

    assert_eq!(state.money, 40);
}

#[test]
fn double_tag_copies_every_pack_tag_effect() {
    let cases = [
        (SkipTag::Standard, BoosterPackKind::Standard),
        (SkipTag::Charm, BoosterPackKind::Arcana),
        (SkipTag::Meteor, BoosterPackKind::Celestial),
        (SkipTag::Buffoon, BoosterPackKind::Buffoon),
    ];

    for (tag, expected_kind) in cases {
        let mut state = GameState::new(tag as u64 + 200);
        apply_tag(&mut state, SkipTag::Double);
        apply_tag(&mut state, tag);

        let mut tags = std::mem::take(&mut state.tags);
        let free_packs = tags.free_packs();
        assert_eq!(
            free_packs.len(),
            2,
            "Double should create two {tag:?} packs"
        );
        assert!(free_packs.iter().all(|pack| {
            pack.kind == expected_kind && pack.size == BoosterPackSize::Mega && pack.price == 0
        }));

        let openings = tags.open_free_packs(&mut state);
        state.tags = tags;
        assert_eq!(openings.len(), 2);
        let expected_options = if expected_kind == BoosterPackKind::Buffoon {
            4
        } else {
            5
        };
        assert!(
            openings
                .iter()
                .all(|opening| opening.options().len() == expected_options)
        );
    }
}

#[test]
fn double_tag_copies_the_spectral_pack_effect() {
    let mut state = GameState::new(2);
    apply_tag(&mut state, SkipTag::Double);
    apply_tag(&mut state, SkipTag::Ethereal);

    let mut tags = std::mem::take(&mut state.tags);
    let free_packs = tags.free_packs();
    assert_eq!(free_packs.len(), 2);
    assert!(free_packs.iter().all(|pack| {
        pack.kind == BoosterPackKind::Spectral
            && pack.size == BoosterPackSize::Normal
            && pack.price == 0
    }));
    let openings = tags.open_free_packs(&mut state);
    state.tags = tags;
    assert_eq!(openings.len(), 2);
    assert!(openings.iter().all(|opening| opening.options().len() == 2));
}

#[test]
fn double_and_ethereal_skips_open_packs_without_leaving_blind_selection() {
    let (mut state, selector_before) = (1..10_000)
        .map(GameState::new)
        .find_map(|state| {
            let selector = state.blind_selector;
            (selector.small().skip_tag == Some(SkipTag::Double)
                && state.blind_selector.big().skip_tag == Some(SkipTag::Ethereal))
            .then_some((state, selector))
        })
        .expect("a deterministic seed should produce the required tag pair");

    state.skip_blind(BlindKind::Small).unwrap();
    assert_eq!(state.blind_selector, selector_before);

    state.skip_blind(BlindKind::Big).unwrap();
    assert_eq!(state.blind_selector, selector_before);
    assert_eq!(state.tag_pack_openings().len(), 2);
    assert_eq!(state.tag_pack_openings()[0].options().len(), 2);

    state.choose_tag_pack(0, &[0]).unwrap();
    assert_eq!(state.tag_pack_openings().len(), 1);
}

#[test]
fn boss_blinds_cannot_be_skipped() {
    let mut state = GameState::new(7);
    let selector_before = state.blind_selector;
    let skipped_before = state.counters.blinds_skipped;

    assert_eq!(
        state.skip_blind(BlindKind::Boss(BossBlinds::Hook)),
        Err(BlindSkipError::BossBlindCannotBeSkipped)
    );
    assert_eq!(state.blind_selector, selector_before);
    assert_eq!(state.counters.blinds_skipped, skipped_before);
    assert!(state.tag_pack_openings().is_empty());
}

#[test]
fn double_tag_creates_two_voucher_offers_for_the_next_voucher_tag() {
    let mut state = GameState::new(5);
    apply_tag(&mut state, SkipTag::Double);
    apply_tag(&mut state, SkipTag::Voucher);
    let mut tags = std::mem::take(&mut state.tags);
    let mut shop = ShopState::new();

    tags.prepare_shop(&mut state, &mut shop);
    state.tags = tags;

    assert_eq!(shop.extra_vouchers.len(), 2);
    assert_ne!(
        shop.extra_vouchers[0].voucher,
        shop.extra_vouchers[1].voucher
    );
}

#[test]
fn multiple_double_tags_add_one_copy_each() {
    let mut state = GameState::new(6);
    apply_tag(&mut state, SkipTag::Double);
    apply_tag(&mut state, SkipTag::Double);
    apply_tag(&mut state, SkipTag::Voucher);
    let mut tags = std::mem::take(&mut state.tags);
    let mut shop = ShopState::new();

    tags.prepare_shop(&mut state, &mut shop);
    state.tags = tags;

    assert_eq!(shop.extra_vouchers.len(), 3);
}

#[test]
fn coupon_tag_makes_initial_shop_items_and_packs_free() {
    let mut state = GameState::new(3);
    apply_tag(&mut state, SkipTag::Coupon);
    let mut tags = std::mem::take(&mut state.tags);
    let mut shop = ShopState::new();
    shop.add_item(ShopItem::PlayingCard {
        card: crate::core::card::Card::new(
            crate::core::enums::Rank::Ace,
            crate::core::enums::Suit::Spades,
            crate::core::enums::Enhancement::None,
            crate::core::enums::Edition::None,
            crate::core::enums::Seal::None,
        ),
        price: 1,
    });
    shop.add_pack(ShopPack::new(
        BoosterPackKind::Standard,
        BoosterPackSize::Normal,
        4,
    ))
    .unwrap();

    tags.prepare_shop(&mut state, &mut shop);
    state.tags = tags;

    assert_eq!(shop.items[0].price(&state), 0);
    assert_eq!(shop.packs[0].price, 0);
}

#[test]
fn investment_tag_pays_at_the_next_boss_payout() {
    let mut state = GameState::new(4);
    state.begin_blind_as(BlindKind::Boss(BossBlinds::Hook));
    state.blind.chips_scored = state.blind.score_requirement;
    apply_tag(&mut state, SkipTag::Investment);

    let payout = state.finish_blind(&Hand::new()).unwrap();

    assert_eq!(state.money, 4 + i16::from(payout.total));
    assert!(state.money >= 25);
}

#[test]
fn tag_state_starts_empty() {
    assert!(TagState::new().held().is_empty());
}
