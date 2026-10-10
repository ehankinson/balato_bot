use crate::core::enums::{PokerHand, SkipTag, Vouchers};
use crate::core::game::GameState;
use crate::core::joker::Joker;
use crate::core::joker_types::{JokerEdition, JokerRarity};
use crate::core::pack::{BoosterPackKind, BoosterPackSize, PackOpening, ShopPack};
use crate::core::shop::{ShopItem, ShopState, ShopVoucher};
use rand::prelude::{IndexedRandom, RngExt};

#[derive(Debug, Clone, Copy)]
struct TaggedJoker {
    rarity: Option<JokerRarity>,
    edition: JokerEdition,
}

/// Tags earned by skipping blinds and the effects waiting for a later phase.
#[derive(Default)]
pub(crate) struct TagState {
    held: Vec<SkipTag>,
    shop_jokers: Vec<TaggedJoker>,
    free_packs: Vec<ShopPack>,
    voucher_next_shop: u8,
    coupon_next_shop: u8,
    d6_next_shop: u8,
    juggle_hand_size: i8,
    double_next_tag: u8,
    investment_pending: u8,
}

impl TagState {
    pub(crate) fn new() -> TagState {
        TagState::default()
    }

    pub(crate) fn held(&self) -> &[SkipTag] {
        &self.held
    }

    pub(crate) fn gain(&mut self, tag: SkipTag, state: &mut GameState) {
        if tag == SkipTag::Double {
            self.double_next_tag = self.double_next_tag.saturating_add(1);
            self.held.push(tag);
            return;
        }

        let copies = self.double_next_tag.saturating_add(1);
        self.double_next_tag = 0;
        for _ in 0..copies {
            self.apply_once(tag, state);
            self.held.push(tag);
        }
    }

    fn apply_once(&mut self, tag: SkipTag, state: &mut GameState) {
        match tag {
            SkipTag::Uncommon => self.shop_jokers.push(TaggedJoker {
                rarity: Some(JokerRarity::Uncommon),
                edition: JokerEdition::None,
            }),
            SkipTag::Rare => self.shop_jokers.push(TaggedJoker {
                rarity: Some(JokerRarity::Rare),
                edition: JokerEdition::None,
            }),
            SkipTag::Negative => self.shop_jokers.push(TaggedJoker {
                rarity: None,
                edition: JokerEdition::Negative,
            }),
            SkipTag::Foil => self.shop_jokers.push(TaggedJoker {
                rarity: Some(JokerRarity::Common),
                edition: JokerEdition::Foil,
            }),
            SkipTag::Holographic => self.shop_jokers.push(TaggedJoker {
                rarity: Some(JokerRarity::Common),
                edition: JokerEdition::Holographic,
            }),
            SkipTag::Polychrome => self.shop_jokers.push(TaggedJoker {
                rarity: Some(JokerRarity::Common),
                edition: JokerEdition::Polychrome,
            }),
            SkipTag::Investment => {
                self.investment_pending = self.investment_pending.saturating_add(1)
            }
            SkipTag::Voucher => self.voucher_next_shop = self.voucher_next_shop.saturating_add(1),
            SkipTag::Boss => state.regenerate_blind_selector(),
            SkipTag::Standard => self.add_mega_pack(BoosterPackKind::Standard),
            SkipTag::Charm => self.add_mega_pack(BoosterPackKind::Arcana),
            SkipTag::Meteor => self.add_mega_pack(BoosterPackKind::Celestial),
            SkipTag::Buffoon => self.add_mega_pack(BoosterPackKind::Buffoon),
            SkipTag::Handy => {
                state.money = state.money.saturating_add(state.blind.hands_played as i16)
            }
            SkipTag::Garbage => {
                state.money = state
                    .money
                    .saturating_add(state.blind.discards_remaining as i16)
            }
            SkipTag::Ethereal => {
                self.add_free_pack(BoosterPackKind::Spectral, BoosterPackSize::Normal)
            }
            SkipTag::Coupon => self.coupon_next_shop = self.coupon_next_shop.saturating_add(1),
            SkipTag::Juggle => self.juggle_hand_size = self.juggle_hand_size.saturating_add(3),
            SkipTag::D6 => self.d6_next_shop = self.d6_next_shop.saturating_add(1),
            SkipTag::TopUp => {
                for _ in 0..2 {
                    if !state.joker_has_room() {
                        break;
                    }
                    if let Some(joker) =
                        Joker::random_with_rarity(JokerRarity::Common, &mut state.rng)
                    {
                        state.jokers.add(joker);
                    }
                }
            }
            SkipTag::Speed => {
                state.money = state
                    .money
                    .saturating_add((state.counters.blinds_skipped as i16).saturating_mul(5));
            }
            SkipTag::Orbital => {
                let hands = [
                    PokerHand::HighCard,
                    PokerHand::Pair,
                    PokerHand::ThreeOfAKind,
                    PokerHand::FourOfAKind,
                    PokerHand::FiveOfAKind,
                    PokerHand::TwoPair,
                    PokerHand::Straight,
                    PokerHand::Flush,
                    PokerHand::FullHouse,
                    PokerHand::StraightFlush,
                    PokerHand::FlushHouse,
                    PokerHand::FlushFive,
                ];
                if let Some(&hand) = hands.choose(&mut state.rng) {
                    for _ in 0..3 {
                        let (chips, mult) = crate::core::consumable::planet_upgrade_values(hand);
                        state.upgrade_hand_score(hand, chips, mult);
                    }
                }
            }
            SkipTag::Economy => {
                state.money = if state.money > 0 {
                    state.money.saturating_mul(2).min(40)
                } else {
                    0
                };
            }
            SkipTag::Double => unreachable!("Double is handled before applying a tag"),
        }
    }

    fn add_mega_pack(&mut self, kind: BoosterPackKind) {
        self.add_free_pack(kind, BoosterPackSize::Mega);
    }

    fn add_free_pack(&mut self, kind: BoosterPackKind, size: BoosterPackSize) {
        self.free_packs.push(ShopPack::new(kind, size, 0));
    }

    pub(crate) fn prepare_shop(&mut self, state: &mut GameState, shop: &mut ShopState) {
        for tagged in self.shop_jokers.drain(..) {
            if !state.joker_has_room() {
                break;
            }
            let mut joker = match tagged.rarity {
                Some(rarity) => Joker::random_with_rarity(rarity, &mut state.rng)
                    .expect("the Joker table must contain every tag rarity"),
                None => Joker::random_shop_joker(&mut state.rng),
            };
            joker.set_edition(tagged.edition);
            shop.add_item(ShopItem::Joker { joker, price: 0 });
        }

        if self.coupon_next_shop > 0 {
            for item in &mut shop.items {
                match item {
                    ShopItem::Joker { price, .. }
                    | ShopItem::Consumable { price, .. }
                    | ShopItem::PlayingCard { price, .. } => *price = 0,
                }
            }
            for pack in &mut shop.packs {
                pack.price = 0;
            }
            self.coupon_next_shop = 0;
        }

        if self.d6_next_shop > 0 {
            shop.reroll_cost = 0;
            self.d6_next_shop = 0;
        }

        if self.voucher_next_shop > 0 {
            let mut eligible = Vouchers::ALL
                .iter()
                .copied()
                .filter(|voucher| state.can_redeem_voucher(*voucher))
                .collect::<Vec<_>>();
            for _ in 0..self.voucher_next_shop {
                if eligible.is_empty() {
                    break;
                }
                let index = state.rng.random_range(0..eligible.len());
                let voucher = eligible.swap_remove(index);
                shop.extra_vouchers.push(ShopVoucher { voucher, price: 10 });
            }
        }
        self.voucher_next_shop = 0;
    }

    pub(crate) fn free_packs(&self) -> &[ShopPack] {
        &self.free_packs
    }

    pub(crate) fn open_free_packs(&mut self, state: &mut GameState) -> Vec<PackOpening> {
        self.free_packs
            .drain(..)
            .map(|pack| pack.open(state))
            .collect()
    }

    pub(crate) fn take_juggle_hand_size(&mut self) -> i8 {
        std::mem::take(&mut self.juggle_hand_size)
    }

    pub(crate) fn take_investment(&mut self) -> u8 {
        std::mem::take(&mut self.investment_pending)
    }
}
