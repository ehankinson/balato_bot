use crate::core::blind::{BlindKind, BlindSkipError, BlindState, ScoreResult};
use crate::core::blind_selector::BlindSelector;
use crate::core::boss_blind::{FinisherBossBlindHistory, RegularBossBlindHistory};
use crate::core::card::{Card, ScoringValues};
use crate::core::consumable::{
    ConsumableTarget, ConsumableUseError, ConsumableUseSource, DEFAULT_CONSUMABLE_PRICE,
    DEFAULT_CONSUMABLE_SLOTS, OwnedConsumable,
};
use crate::core::deck::Deck;
use crate::core::enums::{
    ALL_RANKS, ALL_SUITS, BossBlinds, Consumable, Decks, Edition, Enhancement, Planet, PokerHand,
    Seal, SkipTag, Spectral, Stakes, Tarot, Vouchers,
};
use crate::core::hand::Hand;
use crate::core::joker::{
    GeneratedConsumable, Joker, JokerContext, JokerEffect, JokerOperation, Jokers, TriggerEvent,
    UpdateEvent,
};
use crate::core::pack::{PackError, PackOpening};
use crate::core::shop::{ShopError, ShopState};
use crate::core::tag::TagState;
use crate::core::voucher::VoucherError;
use chacha20::ChaCha12Rng;
use rand::prelude::{IndexedRandom, SliceRandom};
use rand::{RngExt, SeedableRng};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct HandScore {
    pub(crate) chips: u16,
    pub(crate) mult: u16,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct BlindPayout {
    pub(crate) blind_reward: u8,
    pub(crate) interest: i16,
    pub(crate) hand_money: i16,
    pub(crate) discard_money: i16,
    pub(crate) total: i16,
}

pub(crate) const BASE_HAND_SCORES: [HandScore; 12] = [
    HandScore { chips: 5, mult: 1 },
    HandScore { chips: 10, mult: 2 },
    HandScore { chips: 30, mult: 3 },
    HandScore { chips: 60, mult: 7 },
    HandScore {
        chips: 120,
        mult: 12,
    },
    HandScore { chips: 20, mult: 2 },
    HandScore { chips: 30, mult: 4 },
    HandScore { chips: 35, mult: 4 },
    HandScore { chips: 40, mult: 4 },
    HandScore {
        chips: 100,
        mult: 8,
    },
    HandScore {
        chips: 140,
        mult: 14,
    },
    HandScore {
        chips: 160,
        mult: 16,
    },
];

/// Counters that persist across blind transitions and shop phases.
pub(crate) struct GameCounters {
    pub(crate) round: u32,
    pub(crate) ante: u8,
    pub(crate) life: i16,
    pub(crate) cards_sold: u16,
    pub(crate) tarot_cards_used: u32,
    pub(crate) planet_cards_used: u32,
    pub(crate) shop_rerolls: u32,
    pub(crate) boss_rerolls_used: u32,
    pub(crate) blinds_skipped: u32,
    pub(crate) used_planets: u16,
    pub(crate) sell_value_bonus: u8,
    pub(crate) poker_hands_played: [u32; 12],
}

impl GameCounters {
    fn new() -> GameCounters {
        GameCounters {
            round: 1,
            ante: 1,
            life: 1,
            cards_sold: 0,
            tarot_cards_used: 0,
            planet_cards_used: 0,
            shop_rerolls: 0,
            boss_rerolls_used: 0,
            blinds_skipped: 0,
            used_planets: 0,
            sell_value_bonus: 0,
            poker_hands_played: [0; 12],
        }
    }
}

pub struct GameState {
    pub(crate) blind: BlindState,
    pub(crate) blind_selector: BlindSelector,
    pub(crate) deck: Deck,
    pub(crate) jokers: Jokers,
    pub(crate) money: i16,
    pub(crate) interest_cap: i16,
    pub(crate) consumable_capacity: usize,
    pub(crate) shop_discount_percent: u8,
    pub(crate) edition_rate_multiplier: u8,
    pub(crate) boss_rerolls_per_ante: u8,
    pub(crate) boss_reroll_unlimited: bool,
    pub(crate) shop: ShopState,
    pub(crate) vouchers: Vec<Vouchers>,
    pub(crate) consumables: Vec<OwnedConsumable>,
    pub(crate) last_consumable: Option<OwnedConsumable>,
    pub(crate) hand_scores: Vec<HandScore>,
    pub(crate) counters: GameCounters,
    pub(crate) ox_target_hand: PokerHand,
    pub(crate) ante_voucher: Option<Vouchers>,
    pub(crate) tags: TagState,
    pub(crate) tag_pack_openings: Vec<crate::core::pack::PackOpening>,
    regular_boss_blinds: RegularBossBlindHistory,
    finisher_boss_blinds: FinisherBossBlindHistory,
    pub(crate) rng: ChaCha12Rng,
}

/// The point where the game is waiting for the next player or model decision.
///
/// These discriminants are part of the Python control contract. Keep them
/// stable when adding a new decision point; the Python API returns the numeric
/// code instead of a phase name.
#[repr(u8)]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(crate) enum DecisionPoint {
    BlindSelection = 0,
    PlayingHand = 1,
    Shop = 2,
    PackSelection = 3,
    ConsumableSelection = 4,
    RunComplete = 5,
}

impl DecisionPoint {
    pub(crate) const fn code(self) -> u8 {
        self as u8
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum DecisionError {
    InvalidPhase,
    InvalidBlindChoice,
}

/// Action names shared by the Rust flow and the Python control layer.
#[pyo3::pyclass(name = "Action", from_py_object)]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(crate) enum ActionKind {
    SelectBlind = 0,
    SkipBlind = 1,
    PlayCards = 2,
    DiscardCards = 3,
    BuyShopItem = 4,
    BuyVoucher = 5,
    RerollShop = 6,
    BuyPack = 7,
    SelectPackOption = 8,
    UseConsumable = 9,
    SellJoker = 10,
    SellConsumable = 11,
    LeaveShop = 12,
    SkipPack = 13,
    UsePackConsumable = 14,
    ReorderJokers = 15,
    ReorderHand = 16,
    RerollBoss = 17,
}

pub(crate) enum GameAction {
    SelectBlind(BlindKind),
    SkipBlind(BlindKind),
    PlayCards(Vec<usize>),
    DiscardCards(Vec<usize>),
    BuyShopItem(usize),
    BuyVoucher(usize),
    RerollShop,
    BuyPack(usize),
    SelectPackOption(usize),
    UseConsumable {
        index: usize,
        target: ConsumableTarget,
    },
    SellJoker(usize),
    SellConsumable(usize),
    LeaveShop,
    SkipPack,
    UsePackConsumable {
        index: usize,
        target: ConsumableTarget,
    },
    ReorderJokers(Vec<usize>),
    ReorderHand(Vec<usize>),
    RerollBoss,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ActionError {
    InvalidPhase,
    InvalidAction,
    Decision(DecisionError),
    BlindSkip(BlindSkipError),
    Shop(ShopError),
    Pack(PackError),
    Consumable(ConsumableUseError),
}

impl GameState {
    pub fn new(seed: u64) -> GameState {
        GameState::with_deck(seed, Decks::Red)
    }

    pub(crate) fn with_deck(seed: u64, deck_type: Decks) -> GameState {
        Self::with_rules(seed, deck_type, Stakes::White)
    }

    pub(crate) fn with_rules(seed: u64, deck_type: Decks, stake: Stakes) -> GameState {
        let mut rng = ChaCha12Rng::seed_from_u64(seed);

        let deck = Deck::new(deck_type, &mut rng);
        let consumable_capacity =
            (DEFAULT_CONSUMABLE_SLOTS as i8 + deck.consumable_slot_modifier()).max(0) as usize;

        let mut state = GameState {
            blind: BlindState::new(),
            blind_selector: BlindSelector::empty(),
            deck,
            jokers: Jokers::new(),
            money: 4,
            interest_cap: 5,
            consumable_capacity,
            shop_discount_percent: 0,
            edition_rate_multiplier: 1,
            boss_rerolls_per_ante: 0,
            boss_reroll_unlimited: false,
            shop: ShopState::new(),
            vouchers: Vec::new(),
            consumables: Vec::new(),
            last_consumable: None,
            hand_scores: BASE_HAND_SCORES.to_vec(),
            counters: GameCounters::new(),
            ox_target_hand: PokerHand::HighCard,
            ante_voucher: None,
            tags: TagState::new(),
            tag_pack_openings: Vec::new(),
            regular_boss_blinds: RegularBossBlindHistory::new(),
            finisher_boss_blinds: FinisherBossBlindHistory::new(),
            rng,
        };
        state.blind.stake = stake;
        state.blind.ante_scaling = if deck_type == Decks::Plasma { 2 } else { 1 };
        state
            .blind
            .adjust_discards_per_blind(state.deck.discard_modifier());
        if stake as u8 >= Stakes::Blue as u8 {
            state.blind.adjust_discards_per_blind(-1);
        }
        if deck_type == Decks::Blue {
            state.blind.adjust_hands_per_blind(1);
        }
        if deck_type == Decks::Black {
            state.blind.adjust_hands_per_blind(-1);
            state.jokers.increase_capacity(1);
        }
        if deck_type == Decks::Painted {
            state.jokers.decrease_capacity(1);
        }
        match deck_type {
            Decks::Yellow => state.money += 10,
            Decks::Magic => {
                state.redeem_voucher(Vouchers::CrystalBall).unwrap();
                state
                    .add_consumable(Consumable::Tarot(Tarot::Fool))
                    .unwrap();
                state
                    .add_consumable(Consumable::Tarot(Tarot::Fool))
                    .unwrap();
            }
            Decks::Nebula => {
                state.redeem_voucher(Vouchers::Telescope).unwrap();
            }
            Decks::Ghost => {
                state
                    .add_consumable(Consumable::Spectral(Spectral::Hex))
                    .unwrap();
            }
            Decks::Zodiac => {
                for voucher in [
                    Vouchers::TarotMerchant,
                    Vouchers::PlanetMerchant,
                    Vouchers::Overstock,
                ] {
                    state.redeem_voucher(voucher).unwrap();
                }
            }
            _ => {}
        }
        state.shop.apply_deck_rules(deck_type);
        state.blind.hands_remaining = state.blind.hands_per_blind;
        state.blind.discards_remaining = state.blind.discards_per_blind;
        state.regenerate_blind_selector();
        state.roll_ante_voucher();
        state
    }

    pub(crate) fn hand_size(&self) -> i8 {
        self.deck.hand_size() + self.blind.hand_size_modifier + self.jokers.hand_size_modifier()
    }

    pub(crate) fn consumable_capacity(&self) -> usize {
        self.consumable_capacity
            + self
                .consumables
                .iter()
                .filter(|owned| owned.negative)
                .count()
    }

    pub(crate) fn joker_capacity(&self) -> usize {
        self.jokers.capacity()
    }

    pub(crate) fn empty_joker_slots(&self) -> usize {
        self.joker_capacity()
            .saturating_sub(self.jokers.as_slice().len())
    }

    pub(crate) fn has_showman(&self) -> bool {
        self.jokers.modifiers().showman
    }

    pub(crate) fn joker_has_room(&self) -> bool {
        self.jokers.as_slice().len() < self.joker_capacity()
    }

    pub(crate) fn hands_per_blind(&self) -> u8 {
        self.blind.hands_per_blind
    }

    pub(crate) fn discards_per_blind(&self) -> u8 {
        self.blind.discards_per_blind
    }

    pub(crate) fn begin_blind(&mut self) {
        self.begin_blind_as(BlindKind::Small);
    }

    pub(crate) fn begin_random_boss_blind(&mut self) -> BossBlinds {
        let boss = self.random_boss_blind();
        self.begin_blind_as(BlindKind::Boss(boss));
        boss
    }

    pub(crate) fn random_boss_blind(&mut self) -> BossBlinds {
        let candidates: Vec<BossBlinds> = if self.counters.ante >= 8 && self.counters.ante % 8 == 0
        {
            BossBlinds::SHOWDOWN.to_vec()
        } else {
            BossBlinds::ALL
                .into_iter()
                .filter(|boss| boss.available_at_ante(self.counters.ante))
                .collect()
        };
        let minimum = candidates
            .iter()
            .map(|boss| self.boss_appearances(*boss))
            .min()
            .expect("the boss blind pool must not be empty");
        let least_seen = candidates
            .into_iter()
            .filter(|boss| self.boss_appearances(*boss) == minimum)
            .collect::<Vec<_>>();
        let boss = *least_seen
            .choose(&mut self.rng)
            .expect("the least-seen boss blind pool must not be empty");
        self.increment_boss_appearances(boss);
        boss
    }

    fn boss_appearances(&self, boss: BossBlinds) -> u16 {
        if boss.is_showdown() {
            self.finisher_boss_blinds.count(boss)
        } else {
            self.regular_boss_blinds.count(boss)
        }
    }

    fn increment_boss_appearances(&mut self, boss: BossBlinds) {
        if boss.is_showdown() {
            self.finisher_boss_blinds.record(boss);
        } else {
            self.regular_boss_blinds.record(boss);
        }
    }

    pub(crate) fn begin_blind_as(&mut self, kind: BlindKind) {
        self.deck.clear_debuffs();
        self.jokers.clear_blind_effects();
        self.blind.begin(kind, self.counters.ante);
        self.blind.hand_size_modifier += self.tags.take_juggle_hand_size();
        self.update_jokers(UpdateEvent::RoundStarted);
        self.update_jokers(UpdateEvent::BlindSelected {
            is_boss: kind.is_boss(),
        });
        if kind.is_boss() && self.jokers.modifiers().disable_boss {
            self.blind.disable_boss_ability();
        }
        if !self.blind.boss_ability_disabled {
            if let BlindKind::Boss(boss) = kind {
                match boss {
                    BossBlinds::AmberAcorn => {
                        for joker in self.jokers.as_mut_slice() {
                            joker.set_face_down(true);
                        }
                        self.jokers.shuffle(&mut self.rng);
                    }
                    BossBlinds::CrimsonHeart => {
                        self.jokers.disable_random(&mut self.rng);
                    }
                    _ => {}
                }
                if matches!(
                    boss,
                    BossBlinds::Water
                        | BossBlinds::Manacle
                        | BossBlinds::Needle
                        | BossBlinds::Wall
                        | BossBlinds::VioletVessel
                        | BossBlinds::AmberAcorn
                        | BossBlinds::CrimsonHeart
                        | BossBlinds::VerdantLeaf
                ) {
                    self.notify_boss_ability(false);
                }
            }
        }
    }

    pub(crate) fn begin_next_ante(&mut self) {
        self.counters.ante = self.counters.ante.saturating_add(1);
        self.counters.boss_rerolls_used = 0;
        self.deck.clear_play_history();
        self.regenerate_blind_selector();
        self.roll_ante_voucher();
    }

    pub(crate) fn regenerate_blind_selector(&mut self) {
        self.blind_selector = BlindSelector::generate(self);
    }

    pub(crate) fn skip_blind(
        &mut self,
        kind: BlindKind,
    ) -> Result<crate::core::enums::SkipTag, BlindSkipError> {
        if kind.is_boss() {
            return Err(BlindSkipError::BossBlindCannotBeSkipped);
        }

        let tag = match kind {
            BlindKind::Small => self
                .blind_selector
                .small()
                .skip_tag
                .expect("small blinds have skip tags"),
            BlindKind::Big => self
                .blind_selector
                .big()
                .skip_tag
                .expect("big blinds have skip tags"),
            BlindKind::Boss(_) => unreachable!("boss blind skips are rejected above"),
        };
        self.counters.blinds_skipped = self.counters.blinds_skipped.saturating_add(1);
        let mut tags = std::mem::take(&mut self.tags);
        tags.gain(tag, self);
        let openings = tags.open_free_packs(self);
        self.tag_pack_openings.extend(openings);
        self.tags = tags;
        Ok(tag)
    }

    pub(crate) fn tag_pack_openings(&self) -> &[crate::core::pack::PackOpening] {
        &self.tag_pack_openings
    }

    pub(crate) fn finish_blind(&mut self, hand: &Hand) -> Option<BlindPayout> {
        if self.blind.finished || (!self.blind.has_beaten_target() && !self.blind.prevent_death) {
            return None;
        }

        self.blind.finished = true;
        let is_boss = self.blind.kind.is_boss();
        self.update_jokers(UpdateEvent::BlindCompleted { is_boss });
        if is_boss {
            self.update_jokers(UpdateEvent::BossBlindCompleted);
        }
        self.update_jokers(UpdateEvent::RoundCompleted {
            held_cards: hand.cards(),
        });
        self.money -= self.jokers.finish_round_stickers();
        self.apply_held_round_effects(hand);
        let blind_reward = self.blind.reward();
        let green = self.deck.deck_type() == Decks::Green;
        let interest = if green {
            0
        } else {
            (self.money.max(0) / 5).min(self.interest_cap)
        };
        let hand_money = i16::from(self.blind.hands_remaining) * if green { 2 } else { 1 };
        let discard_money = if green {
            i16::from(self.blind.discards_remaining)
        } else {
            0
        };
        let investment = if self.blind.kind.is_boss() {
            let mut tags = std::mem::take(&mut self.tags);
            let reward = i16::from(tags.take_investment()).saturating_mul(25);
            self.tags = tags;
            reward
        } else {
            0
        };
        let total = i16::from(blind_reward) + interest + investment + hand_money + discard_money;
        self.money += total;
        self.counters.round += 1;
        if is_boss && self.deck.deck_type() == Decks::Anaglyph {
            self.materialize_generated_effects(
                &[JokerOperation::AddDoubleTag { count: 1 }],
                &mut [],
                None,
            );
        }
        self.deck.clear_debuffs();
        self.jokers.clear_blind_effects();
        if is_boss {
            self.ox_target_hand = self.most_played_hand().unwrap_or(PokerHand::HighCard);
            self.deck.clear_play_history();
        }
        Some(BlindPayout {
            blind_reward,
            interest,
            hand_money,
            discard_money,
            total,
        })
    }

    fn roll_ante_voucher(&mut self) {
        let available = Vouchers::ALL
            .iter()
            .copied()
            .filter(|voucher| self.can_redeem_voucher(*voucher))
            .collect::<Vec<_>>();
        let lowest_available_tier = available.iter().map(|voucher| voucher.tier()).min();
        self.ante_voucher = available
            .into_iter()
            .filter(|voucher| Some(voucher.tier()) == lowest_available_tier)
            .collect::<Vec<_>>()
            .choose(&mut self.rng)
            .copied();
    }

    pub(crate) fn has_voucher(&self, voucher: Vouchers) -> bool {
        self.vouchers.contains(&voucher)
    }

    fn card_probability_roll(&mut self, probability: f64) -> bool {
        let probability = probability * f64::from(self.jokers.modifiers().probability_multiplier);
        probability >= 1.0 || self.rng.random_bool(probability)
    }

    pub(crate) fn played_card_values(&mut self, card: &Card) -> ScoringValues {
        let mut values = card.scoring_values();
        if card.is_debuffed() {
            return values;
        }
        if card.enhancement() == Enhancement::Lucky {
            let mult_success = self.card_probability_roll(1.0 / 5.0);
            let money_success = self.card_probability_roll(1.0 / 15.0);
            if mult_success {
                values.add_mult += 20;
            }
            if money_success {
                self.money += 20;
            }
            if mult_success || money_success {
                self.update_jokers(UpdateEvent::LuckyCardSucceeded);
            }
        }
        if card.seal() == Seal::Gold {
            self.money += 3;
        }
        values
    }

    fn apply_held_round_effects(&mut self, hand: &Hand) {
        let mime_repetitions = self.jokers.mime_count();
        for card in hand.cards() {
            if card.is_debuffed() {
                continue;
            }
            let repetitions = 1 + mime_repetitions + usize::from(card.seal() == Seal::Red);
            for _ in 0..repetitions {
                if card.enhancement() == Enhancement::Gold {
                    self.money += 3;
                }
                if card.seal() == Seal::Blue {
                    if let Some(last_hand) = self.blind.current_hand {
                        let planet = Planet::ALL
                            .into_iter()
                            .find(|&planet| {
                                crate::core::consumable::planet_hand(planet) == last_hand
                            })
                            .expect("every poker hand has a Planet");
                        self.add_generated_consumable(Consumable::Planet(planet));
                    }
                }
            }
        }
    }

    pub(crate) fn planet_mult_multiplier(&self, hand: PokerHand) -> f32 {
        if !self.has_voucher(Vouchers::Observatory) {
            return 1.0;
        }

        let matching_planets = self
            .consumables
            .iter()
            .filter(|consumable| {
                matches!(consumable.consumable, Consumable::Planet(planet) if crate::core::consumable::planet_hand(planet) == hand)
            })
            .count() as i32;
        1.5_f32.powi(matching_planets)
    }

    pub(crate) fn shop_price(&self, base_price: u16) -> u16 {
        let remaining_percent = 100u32.saturating_sub(self.shop_discount_percent as u32);
        if base_price == 0 {
            return 0;
        }
        ((base_price as u32 * remaining_percent) / 100).max(1) as u16
    }

    pub(crate) fn redeem_voucher(&mut self, voucher: Vouchers) -> Result<(), VoucherError> {
        crate::core::voucher::redeem(self, voucher)
    }

    pub(crate) fn validate_voucher_purchase(&self, index: usize) -> Result<(), ShopError> {
        let (voucher, price) = if index == 0 {
            let offer = self.shop.voucher.as_ref().ok_or(ShopError::InvalidOffer)?;
            (offer.voucher, offer.price)
        } else {
            let offer = self
                .shop
                .extra_vouchers
                .get(index - 1)
                .ok_or(ShopError::InvalidOffer)?;
            (offer.voucher, offer.price)
        };
        if self.vouchers.contains(&voucher) {
            return Err(ShopError::VoucherAlreadyOwned);
        }
        let price = self.shop_price(price);
        crate::core::shop::require_funds(self, price)?;
        if !self.can_redeem_voucher(voucher) {
            return Err(ShopError::VoucherPrerequisiteMissing);
        }
        Ok(())
    }

    pub(crate) fn buy_voucher(&mut self, index: usize) -> Result<(), ShopError> {
        self.validate_voucher_purchase(index)?;
        let offer = if index == 0 {
            self.shop.voucher.as_ref().unwrap()
        } else {
            &self.shop.extra_vouchers[index - 1]
        };
        let voucher = offer.voucher;
        let price = self.shop_price(offer.price);

        self.redeem_voucher(voucher).map_err(|error| match error {
            VoucherError::AlreadyOwned => ShopError::VoucherAlreadyOwned,
            VoucherError::PrerequisiteMissing => ShopError::VoucherPrerequisiteMissing,
        })?;
        if index == 0 {
            self.shop.voucher.take();
        } else {
            self.shop.extra_vouchers.remove(index - 1);
        }
        self.money -= price as i16;

        if matches!(voucher, Vouchers::Overstock | Vouchers::OverstockPlus) {
            let mut shop = std::mem::replace(&mut self.shop, ShopState::new());
            shop.refill_base_items(self);
            self.shop = shop;
        }
        Ok(())
    }

    pub(crate) fn can_redeem_voucher(&self, voucher: Vouchers) -> bool {
        crate::core::voucher::can_redeem(self, voucher)
    }

    pub(crate) fn deck_size(&self) -> u16 {
        self.deck.size()
    }

    pub(crate) fn starting_deck_size(&self) -> u16 {
        self.deck.starting_size()
    }

    pub(crate) fn most_played_hand(&self) -> Option<PokerHand> {
        // Ties prefer the stronger hand, as the game's hand ordering does.
        [
            PokerHand::HighCard,
            PokerHand::Pair,
            PokerHand::TwoPair,
            PokerHand::ThreeOfAKind,
            PokerHand::Straight,
            PokerHand::Flush,
            PokerHand::FullHouse,
            PokerHand::FourOfAKind,
            PokerHand::StraightFlush,
            PokerHand::FiveOfAKind,
            PokerHand::FlushHouse,
            PokerHand::FlushFive,
        ]
        .into_iter()
        .enumerate()
        .filter(|(_, hand)| self.counters.poker_hands_played[*hand as usize - 1] > 0)
        .max_by_key(|(strength, hand)| {
            (
                self.counters.poker_hands_played[*hand as usize - 1],
                *strength,
            )
        })
        .map(|(_, hand)| hand)
    }

    pub(crate) fn hand_score(&self, hand: PokerHand) -> HandScore {
        self.hand_scores[hand as usize - 1]
    }

    pub(crate) fn upgrade_hand_score(&mut self, hand: PokerHand, chips: u16, mult: u16) {
        let score = &mut self.hand_scores[hand as usize - 1];
        score.chips = score.chips.saturating_add(chips);
        score.mult = score.mult.saturating_add(mult);
    }

    pub(crate) fn downgrade_hand_score(&mut self, hand: PokerHand) -> bool {
        let score = &mut self.hand_scores[hand as usize - 1];
        let base = BASE_HAND_SCORES[hand as usize - 1];
        if score.chips <= base.chips && score.mult <= base.mult {
            return false;
        }
        let (chips, mult) = crate::core::consumable::planet_upgrade_values(hand);
        score.chips = score.chips.saturating_sub(chips).max(base.chips);
        score.mult = score.mult.saturating_sub(mult).max(base.mult);
        true
    }

    pub(crate) fn upgrade_all_hand_scores(&mut self) {
        for hand in [
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
        ] {
            let (chips, mult) = crate::core::consumable::planet_upgrade_values(hand);
            self.upgrade_hand_score(hand, chips, mult);
        }
    }

    pub(crate) fn add_consumable(
        &mut self,
        consumable: Consumable,
    ) -> Result<(), ConsumableUseError> {
        self.add_owned_consumable(OwnedConsumable::new(consumable, DEFAULT_CONSUMABLE_PRICE))
    }

    pub(crate) fn add_negative_consumable(
        &mut self,
        consumable: Consumable,
    ) -> Result<(), ConsumableUseError> {
        self.add_owned_consumable(OwnedConsumable::negative(
            consumable,
            DEFAULT_CONSUMABLE_PRICE,
        ))
    }

    pub(crate) fn add_owned_consumable(
        &mut self,
        mut consumable: OwnedConsumable,
    ) -> Result<(), ConsumableUseError> {
        if !consumable.negative && self.consumables.len() >= self.consumable_capacity() {
            return Err(ConsumableUseError::NoCapacity);
        }
        consumable.apply_discount(self.shop_discount_percent);
        self.consumables.push(consumable);
        Ok(())
    }

    pub(crate) fn materialize_generated_effects(
        &mut self,
        operations: &[JokerOperation],
        played_cards: &mut [Card],
        mut held_hand: Option<&mut Hand>,
    ) {
        for operation in operations {
            match *operation {
                JokerOperation::AddPlayedCardChips { index, chips } => {
                    played_cards[index].scoring_values_mut().chips += chips;
                }
                JokerOperation::CopyPlayedCard { index, count } => {
                    for _ in 0..count {
                        if let Some(card) = played_cards.get(index) {
                            held_hand
                                .as_deref_mut()
                                .expect("a played-card copy requires the current hand")
                                .add_card(card.clone());
                            self.update_jokers(UpdateEvent::DeckChanged { cards_added: 1 });
                        }
                    }
                }
                JokerOperation::GenerateJoker { rarity, count } => {
                    for _ in 0..count {
                        if self.joker_has_room() {
                            if let Some(joker) = Joker::random_with_rarity(rarity, &mut self.rng) {
                                self.jokers.add(joker);
                            }
                        }
                    }
                }
                JokerOperation::CopyRandomOwnedJoker { count } => {
                    for _ in 0..count {
                        if self.joker_has_room() {
                            if let Some(joker) =
                                self.jokers.as_slice().choose(&mut self.rng).cloned()
                            {
                                self.jokers.add(joker);
                            }
                        }
                    }
                }
                JokerOperation::GenerateConsumable { kind, count } => {
                    for _ in 0..count {
                        if !self.has_consumable_room() {
                            break;
                        }
                        let pool = match kind {
                            GeneratedConsumable::Tarot => Tarot::ALL
                                .into_iter()
                                .map(Consumable::Tarot)
                                .collect::<Vec<_>>(),
                            GeneratedConsumable::Planet => {
                                Planet::ALL.into_iter().map(Consumable::Planet).collect()
                            }
                            GeneratedConsumable::Spectral => Spectral::ALL
                                .into_iter()
                                .map(Consumable::Spectral)
                                .collect(),
                        };
                        let available = pool
                            .iter()
                            .copied()
                            .filter(|candidate| {
                                self.has_showman()
                                    || !self
                                        .consumables
                                        .iter()
                                        .any(|owned| owned.consumable == *candidate)
                            })
                            .collect::<Vec<_>>();
                        // A fully exhausted Tarot pool falls back to The Fool in Balatro.
                        let consumable =
                            available
                                .choose(&mut self.rng)
                                .copied()
                                .unwrap_or(match kind {
                                    GeneratedConsumable::Tarot => Consumable::Tarot(Tarot::Fool),
                                    GeneratedConsumable::Planet => {
                                        Consumable::Planet(Planet::Pluto)
                                    }
                                    GeneratedConsumable::Spectral => {
                                        Consumable::Spectral(Spectral::Familiar)
                                    }
                                });
                        self.add_generated_consumable(consumable);
                    }
                }
                JokerOperation::GenerateStoneCard { count } => {
                    for _ in 0..count {
                        let rank = *ALL_RANKS
                            .choose(&mut self.rng)
                            .expect("ranks are non-empty");
                        let suit = *ALL_SUITS
                            .choose(&mut self.rng)
                            .expect("suits are non-empty");
                        self.add_generated_card(Card::new(
                            rank,
                            suit,
                            Enhancement::Stone,
                            Edition::None,
                            Seal::None,
                        ));
                    }
                }
                JokerOperation::GenerateSealedCard { count } => {
                    for _ in 0..count {
                        let rank = *ALL_RANKS
                            .choose(&mut self.rng)
                            .expect("ranks are non-empty");
                        let suit = *ALL_SUITS
                            .choose(&mut self.rng)
                            .expect("suits are non-empty");
                        let seal = *[Seal::Gold, Seal::Purple, Seal::Blue, Seal::Red]
                            .choose(&mut self.rng)
                            .expect("seals are non-empty");
                        let mut card =
                            Card::new(rank, suit, Enhancement::None, Edition::None, Seal::None);
                        card.set_seal(seal);
                        self.add_generated_card(card);
                    }
                }
                JokerOperation::EnhancePlayedCard { index, enhancement } => {
                    if let Some(card) = played_cards.get_mut(index) {
                        card.set_enhancement(enhancement);
                        self.update_jokers(UpdateEvent::CardEnhancementAdded { card });
                    }
                }
                JokerOperation::UpgradeHand { hand, levels } => {
                    let (chips, mult) = crate::core::consumable::planet_upgrade_values(hand);
                    for _ in 0..levels {
                        self.upgrade_hand_score(hand, chips, mult);
                    }
                }
                JokerOperation::CopyNegativeConsumable { count } => {
                    for _ in 0..count {
                        if let Some(consumable) = self.consumables.choose(&mut self.rng).copied() {
                            let negative = OwnedConsumable::negative(
                                consumable.consumable,
                                consumable.original_price,
                            );
                            self.add_owned_consumable(negative)
                                .expect("Negative consumables provide their own slot");
                        }
                    }
                }
                JokerOperation::AddDoubleTag { count } => {
                    for _ in 0..count {
                        let mut tags = std::mem::take(&mut self.tags);
                        tags.gain(SkipTag::Double, self);
                        self.tags = tags;
                    }
                }
            }
        }
    }

    pub(crate) fn trigger_joker_at(
        &mut self,
        index: usize,
        event: &TriggerEvent<'_>,
        joker_count: u8,
    ) -> crate::core::joker::JokerEffect {
        let joker_capacity = self.jokers.capacity();
        let modifiers = self.jokers.modifiers();
        let most_played_hand = self.most_played_hand();
        let consumable_slots = self
            .consumable_capacity()
            .saturating_sub(self.consumables.len());
        let GameState {
            blind,
            deck,
            jokers,
            money,
            counters,
            rng,
            ..
        } = self;
        let mut context = JokerContext::from_parts(
            blind,
            deck,
            money,
            counters,
            most_played_hand,
            rng,
            usize::from(joker_count),
            joker_capacity,
            consumable_slots,
            modifiers,
        );
        jokers.trigger_at(index, event, &mut context, joker_count)
    }

    pub(crate) fn trigger_retrigger_joker_at(
        &mut self,
        index: usize,
        event: &TriggerEvent<'_>,
    ) -> u8 {
        let joker_capacity = self.jokers.capacity();
        let modifiers = self.jokers.modifiers();
        let most_played_hand = self.most_played_hand();
        let consumable_slots = self
            .consumable_capacity()
            .saturating_sub(self.consumables.len());
        let joker_count = self.jokers.as_slice().len();
        let GameState {
            blind,
            deck,
            jokers,
            money,
            counters,
            rng,
            ..
        } = self;
        let mut context = JokerContext::from_parts(
            blind,
            deck,
            money,
            counters,
            most_played_hand,
            rng,
            joker_count,
            joker_capacity,
            consumable_slots,
            modifiers,
        );
        jokers
            .trigger_retrigger_at(index, event, &mut context)
            .retriggers
    }

    pub(crate) fn apply_scoring_joker_effect(
        &mut self,
        effect: &crate::core::joker::JokerEffect,
        played_cards: &mut [Card],
        held_hand: &mut Hand,
    ) {
        let joker_capacity = self.jokers.capacity();
        let modifiers = self.jokers.modifiers();
        let most_played_hand = self.most_played_hand();
        let consumable_slots = self
            .consumable_capacity()
            .saturating_sub(self.consumables.len());
        let joker_count = self.jokers.as_slice().len();
        {
            let GameState {
                blind,
                deck,
                jokers,
                money,
                counters,
                rng,
                ..
            } = self;
            let mut context = JokerContext::from_parts(
                blind,
                deck,
                money,
                counters,
                most_played_hand,
                rng,
                joker_count,
                joker_capacity,
                consumable_slots,
                modifiers,
            );
            Jokers::apply_effect_in_context(effect, &mut context, jokers);
        }
        self.materialize_generated_effects(&effect.operations, played_cards, Some(held_hand));
    }

    fn add_generated_consumable(&mut self, consumable: Consumable) {
        let _ = self.add_consumable(consumable);
    }

    fn has_consumable_room(&self) -> bool {
        self.consumables.len() < self.consumable_capacity()
    }

    fn add_generated_card(&mut self, card: Card) {
        self.deck.add_card_to_deck(card);
        self.update_jokers(UpdateEvent::DeckChanged { cards_added: 1 });
    }

    pub(crate) fn sell_joker(&mut self, index: usize) -> Option<u16> {
        let joker = self.jokers.get(index)?;
        if joker.is_eternal() {
            return None;
        }
        let sell_value = joker.sell_value();
        Jokers::dispatch_sale(self, index);
        self.money = self.money.saturating_add(sell_value as i16);
        self.update_jokers(UpdateEvent::AfterCardSold);
        Some(sell_value as u16)
    }

    pub(crate) fn sell_consumable(&mut self, index: usize) -> Option<u16> {
        let consumable = self.consumables.get(index).copied()?;
        let sell_value = consumable.sell_value();
        self.consumables.remove(index);
        self.money = self.money.saturating_add(sell_value as i16);
        self.update_jokers(UpdateEvent::AfterCardSold);
        Some(sell_value)
    }

    pub(crate) fn use_consumable(
        &mut self,
        index: usize,
        hand: &mut Hand,
        target: ConsumableTarget,
    ) -> Result<(), ConsumableUseError> {
        let owned = self
            .consumables
            .get(index)
            .copied()
            .ok_or(ConsumableUseError::ConsumableNotFound)?;
        owned.consumable.apply(
            self,
            hand,
            &target,
            ConsumableUseSource {
                inventory_index: Some(index),
            },
        )?;

        let used = self.consumables.remove(index);
        self.record_consumable_use(used);
        Ok(())
    }

    pub(crate) fn use_pack_consumable(
        &mut self,
        consumable: Consumable,
        hand: &mut Hand,
        target: &ConsumableTarget,
    ) -> Result<(), ConsumableUseError> {
        consumable.apply(self, hand, target, ConsumableUseSource::default())?;
        self.record_consumable_use(OwnedConsumable::new(consumable, DEFAULT_CONSUMABLE_PRICE));
        Ok(())
    }

    fn record_consumable_use(&mut self, used: OwnedConsumable) {
        match used.consumable {
            Consumable::Tarot(Tarot::Fool) => {}
            Consumable::Tarot(_) | Consumable::Planet(_) => {
                self.last_consumable = Some(used);
            }
            Consumable::Spectral(_) => {}
        }

        match used.consumable {
            Consumable::Tarot(_) => {
                self.counters.tarot_cards_used = self.counters.tarot_cards_used.saturating_add(1);
                self.update_jokers(UpdateEvent::TarotCardUsed);
            }
            Consumable::Planet(_) => {
                self.counters.planet_cards_used = self.counters.planet_cards_used.saturating_add(1);
                if let Consumable::Planet(planet) = used.consumable {
                    self.counters.used_planets |= planet.mask();
                }
                self.update_jokers(UpdateEvent::PlanetCardUsed);
            }
            Consumable::Spectral(_) => {}
        }
    }

    pub(crate) fn apply_shop_discount_to_owned_items(&mut self) {
        for joker in self.jokers.as_mut_slice() {
            joker.apply_discount(self.shop_discount_percent);
        }
        for consumable in &mut self.consumables {
            consumable.apply_discount(self.shop_discount_percent);
        }
    }

    pub(crate) fn notify_boss_ability(&mut self, during_hand_scoring: bool) {
        if self.blind.boss_ability_disabled {
            return;
        }
        if let BlindKind::Boss(boss) = self.blind.kind {
            self.trigger_jokers(TriggerEvent::BossBlindAbilityTriggered {
                boss,
                during_hand_scoring,
            });
        }
    }

    pub(crate) fn deal_cards(&mut self, hand: &mut Hand) {
        let retained = hand.cards().len();
        let requested = (self.hand_size().max(0) as usize).saturating_sub(retained);
        let amount = self.blind.draw_amount(requested);
        self.deck.deal_cards(hand, amount, &mut self.rng);
        let modifiers = self.jokers.modifiers();
        // Recompute debuffs on retained cards, but only new draws can be flipped.
        for card in hand.cards_mut().iter_mut().take(retained) {
            card.set_debuffed(self.blind.card_is_debuffed(card, modifiers));
        }
        let mut draw_triggered = self.blind.serpent_draws && hand.cards().len() > retained;
        for card in hand.cards_mut().iter_mut().skip(retained) {
            self.blind
                .prepare_drawn_card(card, &mut self.rng, modifiers);
            draw_triggered |= card.is_face_down() || card.is_debuffed();
        }
        let old_forced = self.blind.forced_card_index;
        self.blind
            .choose_forced_card(hand.cards().len(), &mut self.rng);
        draw_triggered |= self.blind.forced_card_index.is_some() && old_forced.is_none();
        if self.blind.crimson_heart_prepped && !self.blind.boss_ability_disabled {
            draw_triggered |= self.jokers.disable_random(&mut self.rng).is_some();
            self.blind.crimson_heart_prepped = false;
        }
        if draw_triggered {
            self.notify_boss_ability(false);
        }
    }

    pub(crate) fn discard_selected_cards(
        &mut self,
        hand: &mut Hand,
        positions: Vec<usize>,
    ) -> Option<Vec<Card>> {
        if self.blind.discards_remaining == 0 || positions.len() > 5 {
            return None;
        }
        self.discard_selected_cards_with_action(hand, positions, true)
    }

    fn discard_selected_cards_with_action(
        &mut self,
        hand: &mut Hand,
        positions: Vec<usize>,
        counts_as_discard: bool,
    ) -> Option<Vec<Card>> {
        let old_forced = self.blind.forced_card_index;
        let removed_positions = positions.clone();
        let discarded = hand.select_cards(positions);
        if discarded.is_empty() {
            return None;
        }

        self.blind.forced_card_index = old_forced
            .filter(|index| !removed_positions.contains(index))
            .map(|index| {
                index
                    - removed_positions
                        .iter()
                        .filter(|&&position| position < index)
                        .count()
            });
        for card in &discarded {
            if !card.is_debuffed() && card.seal() == Seal::Purple {
                self.materialize_generated_effects(
                    &[JokerOperation::GenerateConsumable {
                        kind: GeneratedConsumable::Tarot,
                        count: 1,
                    }],
                    &mut [],
                    None,
                );
            }
        }
        self.update_jokers(UpdateEvent::CardsDiscarded { cards: &discarded });
        let effect = self.trigger_jokers(TriggerEvent::Discard {
            cards: &discarded,
            is_discard_action: counts_as_discard,
        });
        if effect.destroy_discarded_card {
            self.update_jokers(UpdateEvent::CardDestroyed {
                card: &discarded[0],
            });
        } else {
            self.deck.discard_cards(discarded.clone());
        }
        self.update_jokers(UpdateEvent::DiscardActionCompleted);
        if counts_as_discard {
            self.blind.record_discard();
            self.deal_cards(hand);
        }
        Some(discarded)
    }

    pub(crate) fn apply_hook_discard(&mut self, hand: &mut Hand) -> Vec<Card> {
        let amount = self
            .blind
            .forced_discard_count()
            .min(hand.hand_size() as usize);
        if amount == 0 {
            return Vec::new();
        }

        let mut positions = (0..hand.hand_size() as usize).collect::<Vec<_>>();
        positions.shuffle(&mut self.rng);
        positions.truncate(amount);
        positions.sort_unstable();
        self.discard_selected_cards_with_action(hand, positions, false)
            .map(|discarded| {
                self.notify_boss_ability(false);
                discarded
            })
            .unwrap_or_default()
    }

    pub(crate) fn play_hand(
        &mut self,
        hand: &mut Hand,
        mut selected_positions: Vec<usize>,
    ) -> Option<ScoreResult> {
        selected_positions.sort_unstable();
        let selected_indices = selected_positions.iter().copied().collect::<Vec<_>>();
        let mut played_cards = Vec::new();
        let score =
            crate::core::blind::execute_hand(self, hand, &selected_indices, &mut played_cards)?;
        if played_cards.is_empty() {
            return None;
        }
        let scoring_card_indices = selected_indices
            .iter()
            .enumerate()
            .filter_map(|(index, hand_index)| {
                score
                    .scoring_data
                    .scoring_played_cards
                    .iter()
                    .any(|card| card.index == *hand_index)
                    .then_some(index)
            })
            .collect::<Vec<_>>();
        self.update_jokers(UpdateEvent::HandCompleted {
            played_cards: &played_cards,
            scoring_card_indices: &scoring_card_indices,
            hand_type: Some(score.hand),
            hands_remaining: self.blind.hands_remaining,
        });
        // Glass rolls once per scoring card, after its complete scoring/retrigger sequence.
        let surviving_cards = played_cards
            .into_iter()
            .enumerate()
            .filter_map(|(index, card)| {
                if scoring_card_indices.contains(&index)
                    && !card.is_debuffed()
                    && card.enhancement() == Enhancement::Glass
                    && self.card_probability_roll(1.0 / 4.0)
                {
                    self.update_jokers(UpdateEvent::CardDestroyed { card: &card });
                    None
                } else {
                    Some(card)
                }
            })
            .collect();
        self.deck.discard_cards(surviving_cards);
        self.apply_hook_discard(hand);
        self.blind.chips_scored = self
            .blind
            .chips_scored
            .saturating_add(score.score.max(0.0) as u64);
        // A winning/last hand ends with the cards actually held during that play.
        if !self.blind.has_beaten_target() && self.blind.hands_remaining > 0 {
            self.deal_cards(hand);
        }
        self.resolve_blind_end(hand);
        Some(score)
    }

    pub(crate) fn resolve_blind_end(&mut self, hand: &mut Hand) {
        if self.blind.finished {
            return;
        }
        if !self.blind.has_beaten_target()
            && (self.blind.hands_remaining == 0 || hand.cards().is_empty())
        {
            if !self.blind.prevent_death {
                self.update_jokers(UpdateEvent::AfterPlayerDeath {
                    chips_scored: self.blind.chips_scored,
                    required_chips: self.blind.score_requirement,
                });
            }
        }
        if self.blind.has_beaten_target() || self.blind.prevent_death {
            self.finish_blind(hand);
            if self.blind.kind.is_boss() {
                for card in hand.cards_mut() {
                    card.clear_play_history();
                }
            }
            for card in hand.cards_mut() {
                card.set_debuffed(false);
                card.set_face_down(false);
            }
        }
    }

    pub(crate) fn trigger_jokers(&mut self, event: TriggerEvent<'_>) -> JokerEffect {
        let joker_count = self.jokers.as_slice().len();
        let joker_capacity = self.jokers.capacity();
        let modifiers = self.jokers.modifiers();
        let most_played_hand = self.most_played_hand();
        let consumable_slots = self
            .consumable_capacity()
            .saturating_sub(self.consumables.len());
        let effect = {
            let GameState {
                blind,
                deck,
                jokers,
                money,
                counters,
                rng,
                ..
            } = self;
            let mut context = JokerContext::from_parts(
                blind,
                deck,
                money,
                counters,
                most_played_hand,
                rng,
                joker_count,
                joker_capacity,
                consumable_slots,
                modifiers,
            );
            jokers.trigger_in_context(event, &mut context)
        };
        self.materialize_generated_effects(&effect.operations, &mut [], None);
        effect
    }

    pub(crate) fn update_jokers(&mut self, event: UpdateEvent<'_>) -> JokerEffect {
        Jokers::dispatch_updates(self, event)
    }
}

/// The runtime object that will coordinate phase transitions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BlindStage {
    Small,
    Big,
    Boss,
}

pub(crate) const BOSS_REROLL_COST: u16 = 10;

pub struct Game {
    pub(crate) state: GameState,
    pub(crate) hand: Hand,
    decision_point: DecisionPoint,
    shop_active: bool,
    pending_pack: Option<PendingPack>,
    blind_stage: BlindStage,
}

pub(crate) struct PendingPack {
    pub(crate) opening: PackOpening,
    pub(crate) previous_state: DecisionPoint,
}

impl Game {
    pub fn new(seed: u64) -> Game {
        Self::with_rules(seed, Decks::Red, Stakes::White)
    }

    pub(crate) fn with_rules(seed: u64, deck: Decks, stake: Stakes) -> Game {
        Game {
            state: GameState::with_rules(seed, deck, stake),
            hand: Hand::new(),
            decision_point: DecisionPoint::BlindSelection,
            shop_active: false,
            pending_pack: None,
            blind_stage: BlindStage::Small,
        }
    }

    pub(crate) fn decision_point(&self) -> DecisionPoint {
        self.decision_point
    }

    pub(crate) fn state(&self) -> &GameState {
        &self.state
    }

    pub(crate) fn hand(&self) -> &Hand {
        &self.hand
    }

    /// Advance automatic game work until the current decision boundary.
    ///
    /// The initial setup is completed by `Game::new`, so the first call
    /// reaches the blind-selection decision immediately. Later transitions
    /// will perform their automatic work here before returning the next
    /// decision point.
    pub(crate) fn next_step(&mut self) -> DecisionPoint {
        if self.decision_point == DecisionPoint::BlindSelection {
            self.open_next_tag_pack();
        }
        self.decision_point
    }

    fn open_next_tag_pack(&mut self) {
        if self.pending_pack.is_none() && !self.state.tag_pack_openings.is_empty() {
            let opening = self.state.tag_pack_openings.remove(0);
            self.begin_pack(opening, DecisionPoint::BlindSelection);
        }

        if self.pending_pack.is_some() {
            self.decision_point = DecisionPoint::PackSelection;
        }
    }

    pub(crate) fn apply_action(
        &mut self,
        action: GameAction,
    ) -> Result<DecisionPoint, ActionError> {
        self.validate_action(&action)?;
        match action {
            GameAction::SelectBlind(kind) => {
                self.select_blind(kind).map_err(ActionError::Decision)?;
            }
            GameAction::SkipBlind(kind) => {
                if self.decision_point != DecisionPoint::BlindSelection {
                    return Err(ActionError::InvalidPhase);
                }
                if kind != self.available_blind() {
                    return Err(ActionError::Decision(DecisionError::InvalidBlindChoice));
                }
                self.state
                    .skip_blind(kind)
                    .map_err(ActionError::BlindSkip)?;
                self.advance_blind_stage();
                self.open_next_tag_pack();
            }
            GameAction::PlayCards(positions) => {
                if self.decision_point != DecisionPoint::PlayingHand {
                    return Err(ActionError::InvalidPhase);
                }
                self.state
                    .play_hand(&mut self.hand, positions)
                    .ok_or(ActionError::InvalidAction)?;
                if self.state.blind.finished {
                    self.enter_shop().map_err(ActionError::Shop)?;
                } else if self.state.blind.hands_remaining == 0 || self.hand.cards().is_empty() {
                    self.state.counters.life = 0;
                    self.decision_point = DecisionPoint::RunComplete;
                }
            }
            GameAction::DiscardCards(positions) => {
                if self.decision_point != DecisionPoint::PlayingHand {
                    return Err(ActionError::InvalidPhase);
                }
                self.state
                    .discard_selected_cards(&mut self.hand, positions)
                    .ok_or(ActionError::InvalidAction)?;
                self.state.resolve_blind_end(&mut self.hand);
                if self.state.blind.finished {
                    self.enter_shop().map_err(ActionError::Shop)?;
                } else if self.hand.cards().is_empty() {
                    self.state.counters.life = 0;
                    self.decision_point = DecisionPoint::RunComplete;
                }
            }
            GameAction::BuyShopItem(index) => {
                self.require_shop()?;
                let mut shop = std::mem::replace(&mut self.state.shop, ShopState::new());
                let result = shop.buy_item(index, &mut self.state);
                self.state.shop = shop;
                result.map_err(ActionError::Shop)?;
            }
            GameAction::BuyVoucher(index) => {
                self.require_shop()?;
                self.state.buy_voucher(index).map_err(ActionError::Shop)?;
            }
            GameAction::RerollShop => {
                self.require_shop()?;
                let reroll_cost = self.state.shop.reroll_cost;
                crate::core::shop::require_funds(&self.state, reroll_cost)
                    .map_err(ActionError::Shop)?;
                let mut shop = std::mem::replace(&mut self.state.shop, ShopState::new());
                let result = shop.reroll(&mut self.state);
                self.state.shop = shop;
                result.map_err(ActionError::Shop)?;
            }
            GameAction::BuyPack(index) => {
                self.require_shop()?;
                let mut shop = std::mem::replace(&mut self.state.shop, ShopState::new());
                let result = shop.buy_and_open_pack(index, &mut self.state);
                self.state.shop = shop;
                self.begin_pack(result.map_err(ActionError::Shop)?, DecisionPoint::Shop);
            }
            GameAction::SelectPackOption(index) => {
                self.pending_pack
                    .as_mut()
                    .unwrap()
                    .opening
                    .select(index, &mut self.state)
                    .map_err(ActionError::Pack)?;
                self.finish_pack_if_done();
            }
            GameAction::UsePackConsumable { index, target } => {
                let pending = self.pending_pack.as_mut().unwrap();
                let mut empty = Hand::new();
                let hand = if pending.opening.has_hand() {
                    &mut self.hand
                } else {
                    &mut empty
                };
                pending
                    .opening
                    .use_consumable(index, &target, &mut self.state, hand)
                    .map_err(ActionError::Pack)?;
                self.finish_pack_if_done();
            }
            GameAction::SkipPack => {
                self.pending_pack
                    .as_mut()
                    .unwrap()
                    .opening
                    .skip(&mut self.state)
                    .map_err(ActionError::Pack)?;
                self.finish_pack_if_done();
            }
            GameAction::UseConsumable { index, target } => {
                if self.has_active_hand() {
                    self.state
                        .use_consumable(index, &mut self.hand, target)
                        .map_err(ActionError::Consumable)?;
                } else {
                    self.state
                        .use_consumable(index, &mut Hand::new(), target)
                        .map_err(ActionError::Consumable)?;
                }
                if self.decision_point == DecisionPoint::PlayingHand {
                    if self.hand.cards().is_empty() && self.state.blind.hands_remaining > 0 {
                        self.state.deal_cards(&mut self.hand);
                    }
                    let modifiers = self.state.jokers.modifiers();
                    for card in self.hand.cards_mut() {
                        card.set_debuffed(self.state.blind.card_is_debuffed(card, modifiers));
                    }
                    self.state
                        .blind
                        .choose_forced_card(self.hand.cards().len(), &mut self.state.rng);
                    self.state.resolve_blind_end(&mut self.hand);
                    if self.state.blind.finished {
                        self.enter_shop().map_err(ActionError::Shop)?;
                    } else if self.hand.cards().is_empty() || self.state.blind.hands_remaining == 0
                    {
                        self.state.counters.life = 0;
                        self.decision_point = DecisionPoint::RunComplete;
                    }
                }
            }
            GameAction::SellJoker(index) => {
                self.state
                    .sell_joker(index)
                    .ok_or(ActionError::InvalidAction)?;
                if self.decision_point == DecisionPoint::PlayingHand {
                    let modifiers = self.state.jokers.modifiers();
                    for card in self.hand.cards_mut() {
                        card.set_debuffed(self.state.blind.card_is_debuffed(card, modifiers));
                    }
                    if self.state.blind.boss_ability_disabled
                        && self.hand.hand_size() < self.state.hand_size().max(0) as usize
                    {
                        self.state.deal_cards(&mut self.hand);
                    }
                }
            }
            GameAction::SellConsumable(index) => {
                self.state
                    .sell_consumable(index)
                    .ok_or(ActionError::InvalidAction)?;
            }
            GameAction::LeaveShop => {
                self.leave_shop().ok_or(ActionError::InvalidPhase)?;
            }
            GameAction::ReorderJokers(order) => {
                assert!(
                    Jokers::reorder(&mut self.state, &order),
                    "validated permutation"
                );
            }
            GameAction::ReorderHand(order) => {
                let forced = self.state.blind.forced_card_index;
                assert!(self.hand.reorder(&order), "validated permutation");
                if self.decision_point == DecisionPoint::PlayingHand {
                    self.state.blind.forced_card_index =
                        forced.and_then(|index| order.iter().position(|old| *old == index));
                }
            }
            GameAction::RerollBoss => {
                let boss = self.state.random_boss_blind();
                self.state.blind_selector.set_boss(boss);
                self.state.money -= BOSS_REROLL_COST as i16;
                self.state.counters.boss_rerolls_used =
                    self.state.counters.boss_rerolls_used.saturating_add(1);
            }
        }

        Ok(self.decision_point)
    }

    pub(crate) fn pending_pack(&self) -> Option<&PendingPack> {
        self.pending_pack.as_ref()
    }

    pub(crate) fn has_active_hand(&self) -> bool {
        self.decision_point == DecisionPoint::PlayingHand
            || (self.decision_point == DecisionPoint::PackSelection
                && self
                    .pending_pack
                    .as_ref()
                    .is_some_and(|pending| pending.opening.has_hand()))
    }

    pub(crate) fn active_hand(&self) -> Option<&Hand> {
        self.has_active_hand().then_some(&self.hand)
    }

    fn reconcile_hand(&mut self) {
        let mut cards = self.hand.take_cards();
        for card in &mut cards {
            card.set_face_down(false);
            card.set_debuffed(false);
        }
        self.state.deck.discard_cards(cards);
        self.state.deck.reset_deck();
    }

    fn begin_pack(&mut self, opening: PackOpening, previous_state: DecisionPoint) {
        if opening.has_hand() {
            self.reconcile_hand();
            let size = (self.state.deck.hand_size() + self.state.jokers.hand_size_modifier()).max(0)
                as usize;
            self.state
                .deck
                .deal_cards(&mut self.hand, size, &mut self.state.rng);
        }
        self.pending_pack = Some(PendingPack {
            opening,
            previous_state,
        });
        self.decision_point = DecisionPoint::PackSelection;
    }

    fn finish_pack_if_done(&mut self) {
        let pending = self.pending_pack.as_ref().expect("active pack");
        if pending.opening.choices() != 0 {
            return;
        }
        let previous = pending.previous_state;
        if pending.opening.has_hand() {
            self.reconcile_hand();
        }
        self.pending_pack = None;
        self.decision_point = previous;
        if previous == DecisionPoint::BlindSelection {
            self.open_next_tag_pack();
        }
    }

    pub(crate) fn validate_boss_reroll(&self) -> Result<(), ActionError> {
        if self.decision_point != DecisionPoint::BlindSelection {
            return Err(ActionError::InvalidPhase);
        }
        if !self.state.boss_reroll_unlimited
            && self.state.counters.boss_rerolls_used >= u32::from(self.state.boss_rerolls_per_ante)
        {
            return Err(ActionError::InvalidAction);
        }
        crate::core::shop::require_funds(&self.state, BOSS_REROLL_COST).map_err(ActionError::Shop)
    }

    fn require_shop(&self) -> Result<(), ActionError> {
        if self.decision_point == DecisionPoint::Shop && self.shop_active {
            Ok(())
        } else {
            Err(ActionError::InvalidPhase)
        }
    }

    pub(crate) fn available_blind(&self) -> BlindKind {
        match self.blind_stage {
            BlindStage::Small => BlindKind::Small,
            BlindStage::Big => BlindKind::Big,
            BlindStage::Boss => self.state.blind_selector.boss().kind,
        }
    }

    fn advance_blind_stage(&mut self) {
        self.blind_stage = match self.blind_stage {
            BlindStage::Small => BlindStage::Big,
            BlindStage::Big => BlindStage::Boss,
            BlindStage::Boss => {
                self.state.begin_next_ante();
                BlindStage::Small
            }
        };
    }

    pub(crate) fn select_blind(&mut self, kind: BlindKind) -> Result<(), DecisionError> {
        if self.decision_point != DecisionPoint::BlindSelection {
            return Err(DecisionError::InvalidPhase);
        }

        if kind != self.available_blind() {
            return Err(DecisionError::InvalidBlindChoice);
        }

        let retained = self.hand.take_cards();
        self.state.deck.discard_cards(retained);
        self.state.deck.reset_deck();
        self.state.begin_blind_as(kind);
        self.state.deal_cards(&mut self.hand);
        self.state.resolve_blind_end(&mut self.hand);
        self.decision_point = if (self.hand.cards().is_empty()
            || self.state.blind.hands_remaining == 0)
            && !self.state.blind.finished
        {
            self.state.counters.life = 0;
            DecisionPoint::RunComplete
        } else {
            DecisionPoint::PlayingHand
        };
        Ok(())
    }

    pub(crate) fn enter_shop(&mut self) -> Result<(), ShopError> {
        if self.shop_active {
            return Err(ShopError::ShopAlreadyActive);
        }
        let mut shop = std::mem::replace(&mut self.state.shop, ShopState::new());
        shop.begin_shop();
        shop.apply_deck_rules(self.state.deck.deck_type());
        shop.refresh_base_items(&mut self.state);
        shop.refresh_packs(&mut self.state);
        shop.refresh_voucher(&mut self.state);
        let mut tags = std::mem::take(&mut self.state.tags);
        tags.prepare_shop(&mut self.state, &mut shop);
        self.state.tags = tags;
        self.state.shop = shop;
        self.shop_active = true;
        self.decision_point = DecisionPoint::Shop;
        Ok(())
    }

    pub(crate) fn shop(&self) -> Option<&ShopState> {
        (self.shop_active && self.decision_point == DecisionPoint::Shop).then_some(&self.state.shop)
    }

    pub(crate) fn shop_mut(&mut self) -> Option<&mut ShopState> {
        (self.shop_active && self.decision_point == DecisionPoint::Shop)
            .then_some(&mut self.state.shop)
    }

    pub(crate) fn leave_shop(&mut self) -> Option<()> {
        if !self.shop_active {
            return None;
        }
        self.state.update_jokers(UpdateEvent::ShopClosed);
        self.shop_active = false;
        if self.state.blind.finished
            && matches!(
                (self.blind_stage, self.state.blind.kind),
                (BlindStage::Small, BlindKind::Small)
                    | (BlindStage::Big, BlindKind::Big)
                    | (BlindStage::Boss, BlindKind::Boss(_))
            )
        {
            self.advance_blind_stage();
        }
        self.decision_point = DecisionPoint::BlindSelection;
        Some(())
    }
}

#[cfg(test)]
#[path = "tests/game_state.rs"]
mod tests;

#[cfg(test)]
#[path = "tests/card_deck.rs"]
mod card_deck_tests;

#[cfg(test)]
#[path = "tests/consumable.rs"]
mod consumable_tests;

#[cfg(test)]
#[path = "tests/shop.rs"]
mod shop_tests;

#[cfg(test)]
#[path = "tests/pack.rs"]
mod pack_tests;

#[cfg(test)]
#[path = "tests/voucher.rs"]
mod voucher_tests;

#[cfg(test)]
#[path = "tests/tag.rs"]
mod tag_tests;

#[cfg(test)]
#[path = "tests/game_flow.rs"]
mod game_flow_tests;

#[cfg(test)]
#[path = "tests/action_interface.rs"]
mod action_interface_tests;

#[cfg(test)]
#[path = "tests/joker_modifiers.rs"]
mod joker_modifier_tests;

#[cfg(test)]
#[path = "tests/boss_blind.rs"]
mod boss_blind_tests;

#[cfg(test)]
#[path = "tests/card_scoring.rs"]
mod card_scoring_tests;

#[cfg(test)]
#[path = "tests/joker_economy_flow.rs"]
mod joker_economy_flow_tests;

#[cfg(test)]
#[path = "tests/run_rules.rs"]
mod run_rules_tests;

#[cfg(test)]
#[path = "tests/catalogue_scoring.rs"]
mod catalogue_scoring_tests;

#[cfg(test)]
#[path = "tests/catalogue_ownership.rs"]
mod catalogue_ownership_tests;

#[cfg(test)]
#[path = "tests/catalogue_packs.rs"]
mod catalogue_packs_tests;
