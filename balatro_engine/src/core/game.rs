use crate::core::blind::{BlindKind, BlindSkipError, BlindState, ScoreResult};
use crate::core::blind_selector::BlindSelector;
use crate::core::boss_blind::{FinisherBossBlindHistory, RegularBossBlindHistory};
use crate::core::card::Card;
use crate::core::consumable::{
    ConsumableTarget, ConsumableUseError, DEFAULT_CONSUMABLE_PRICE, DEFAULT_CONSUMABLE_SLOTS,
    OwnedConsumable,
};
use crate::core::deck::Deck;
use crate::core::enums::{BossBlinds, Consumable, Decks, PokerHand, Rank, Tarot, Vouchers};
use crate::core::hand::Hand;
use crate::core::joker::{JokerEffect, Jokers, TriggerEvent, UpdateEvent};
use crate::core::pack::PackError;
use crate::core::shop::{ShopError, ShopState};
use crate::core::tag::TagState;
use crate::core::voucher::VoucherError;
use rand::SeedableRng;
use rand::prelude::{IndexedRandom, SliceRandom};
use rand::rngs::StdRng;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct HandScore {
    pub(crate) chips: u16,
    pub(crate) mult: u16,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct BlindPayout {
    pub(crate) blind_reward: u8,
    pub(crate) interest: i16,
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
    pub(crate) blinds_skipped: u32,
    pub(crate) used_planets: u16,
    pub(crate) sell_value_bonus: u8,
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
            blinds_skipped: 0,
            used_planets: 0,
            sell_value_bonus: 0,
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
    pub(crate) most_played_hand: Option<PokerHand>,
    pub(crate) ante_voucher: Option<Vouchers>,
    pub(crate) tags: TagState,
    pub(crate) tag_pack_openings: Vec<crate::core::pack::PackOpening>,
    pub(crate) pillar_card_ids: Vec<u16>,
    regular_boss_blinds: RegularBossBlindHistory,
    finisher_boss_blinds: FinisherBossBlindHistory,
    pub(crate) rng: StdRng,
}

impl GameState {
    pub fn new(seed: u64) -> GameState {
        GameState::with_deck(seed, Decks::Red)
    }

    pub(crate) fn with_deck(seed: u64, deck_type: Decks) -> GameState {
        let mut rng = StdRng::seed_from_u64(seed);

        let deck = Deck::new(deck_type, &mut rng);
        let consumable_capacity =
            (DEFAULT_CONSUMABLE_SLOTS as i8 + deck.consumable_slot_modifier()).max(0) as usize;

        let mut state = GameState {
            blind: BlindState::new(),
            blind_selector: BlindSelector::empty(),
            deck,
            jokers: Jokers::new(),
            money: 0,
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
            most_played_hand: None,
            ante_voucher: None,
            tags: TagState::new(),
            tag_pack_openings: Vec::new(),
            pillar_card_ids: Vec::new(),
            regular_boss_blinds: RegularBossBlindHistory::new(),
            finisher_boss_blinds: FinisherBossBlindHistory::new(),
            rng,
        };
        state
            .blind
            .adjust_discards_per_blind(state.deck.discard_modifier());
        state.blind.discards_remaining = state.blind.discards_per_blind;
        state.regenerate_blind_selector();
        state.roll_ante_voucher();
        state
    }

    pub(crate) fn hand_size(&self) -> i8 {
        self.deck.hand_size() + self.blind.hand_size_modifier
    }

    pub(crate) fn consumable_capacity(&self) -> usize {
        self.consumable_capacity
    }

    pub(crate) fn joker_capacity(&self) -> usize {
        self.jokers.capacity()
    }

    pub(crate) fn empty_joker_slots(&self) -> usize {
        self.joker_capacity()
            .saturating_sub(self.jokers.as_slice().len())
    }

    pub(crate) fn unique_planet_cards_used(&self) -> u32 {
        self.counters.used_planets.count_ones()
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
        self.jokers.clear_debuffs();
        self.blind.begin(kind, self.counters.ante);
        self.blind.hand_size_modifier += self.tags.take_juggle_hand_size();
        if let BlindKind::Boss(boss) = kind {
            match boss {
                crate::core::enums::BossBlinds::AmberAcorn if !self.blind.boss_ability_disabled => {
                    self.jokers.shuffle(&mut self.rng);
                }
                crate::core::enums::BossBlinds::CrimsonHeart
                    if !self.blind.boss_ability_disabled =>
                {
                    self.jokers.disable_random(&mut self.rng);
                }
                _ => {}
            }
        }
    }

    pub(crate) fn begin_next_ante(&mut self) {
        self.counters.ante = self.counters.ante.saturating_add(1);
        self.pillar_card_ids.clear();
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
            BlindKind::Small => self.blind_selector.small().skip_tag,
            BlindKind::Big => self.blind_selector.big().skip_tag,
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

    pub(crate) fn choose_tag_pack(
        &mut self,
        index: usize,
        selections: &[usize],
    ) -> Result<(), PackError> {
        if index >= self.tag_pack_openings.len() {
            return Err(PackError::InvalidSelection);
        }
        let opening = self.tag_pack_openings.remove(index);
        opening.choose(selections, self)
    }

    pub(crate) fn finish_blind(&mut self) -> Option<BlindPayout> {
        if self.blind.finished || !self.blind.has_beaten_target() {
            return None;
        }

        self.blind.finished = true;
        let blind_reward = self.blind.reward();
        let interest = (self.money.max(0) / 5).min(self.interest_cap);
        let investment = if self.blind.kind.is_boss() {
            let mut tags = std::mem::take(&mut self.tags);
            let reward = i16::from(tags.take_investment()).saturating_mul(25);
            self.tags = tags;
            reward
        } else {
            0
        };
        let total = i16::from(blind_reward) + interest + investment;
        self.money += total;
        self.update_jokers(UpdateEvent::BlindCompleted {
            is_boss: self.blind.kind.is_boss(),
        });
        self.deck.clear_debuffs();
        self.pillar_card_ids.clear();
        Some(BlindPayout {
            blind_reward,
            interest,
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

    pub(crate) fn buy_shop_voucher(&mut self) -> Result<(), ShopError> {
        let (voucher, price) = {
            let offer = self.shop.voucher.as_ref().ok_or(ShopError::InvalidOffer)?;
            (offer.voucher, offer.price)
        };
        if self.vouchers.contains(&voucher) {
            return Err(ShopError::VoucherAlreadyOwned);
        }
        let price = self.shop_price(price);
        if self.money < price as i16 {
            return Err(ShopError::InsufficientFunds);
        }

        self.redeem_voucher(voucher).map_err(|error| match error {
            VoucherError::AlreadyOwned => ShopError::VoucherAlreadyOwned,
            VoucherError::PrerequisiteMissing => ShopError::VoucherPrerequisiteMissing,
        })?;
        self.shop.voucher.take();
        self.money -= price as i16;

        if matches!(voucher, Vouchers::Overstock | Vouchers::OverstockPlus) {
            let mut shop = std::mem::replace(&mut self.shop, ShopState::new());
            shop.refill_base_items(self);
            self.shop = shop;
        }
        Ok(())
    }

    pub(crate) fn buy_extra_shop_voucher(&mut self, index: usize) -> Result<(), ShopError> {
        let offer = self
            .shop
            .extra_vouchers
            .get(index)
            .ok_or(ShopError::InvalidOffer)?;
        if self.vouchers.contains(&offer.voucher) {
            return Err(ShopError::VoucherAlreadyOwned);
        }
        let price = self.shop_price(offer.price);
        if self.money < price as i16 {
            return Err(ShopError::InsufficientFunds);
        }
        let voucher = offer.voucher;
        self.redeem_voucher(voucher).map_err(|error| match error {
            VoucherError::AlreadyOwned => ShopError::VoucherAlreadyOwned,
            VoucherError::PrerequisiteMissing => ShopError::VoucherPrerequisiteMissing,
        })?;
        self.shop.extra_vouchers.remove(index);
        self.money -= price as i16;
        Ok(())
    }

    pub(crate) fn can_redeem_voucher(&self, voucher: Vouchers) -> bool {
        crate::core::voucher::can_redeem(self, voucher)
    }

    pub(crate) fn choose_pack(
        &mut self,
        opening: crate::core::pack::PackOpening,
        selections: &[usize],
    ) -> Result<(), PackError> {
        opening.choose(selections, self)
    }

    pub(crate) fn deck_size(&self) -> u16 {
        self.deck.size()
    }

    pub(crate) fn starting_deck_size(&self) -> u16 {
        self.deck.starting_size()
    }

    pub(crate) fn enhanced_cards(&self) -> u16 {
        self.deck.enhanced_count()
    }

    pub(crate) fn nines_in_deck(&self) -> u16 {
        self.deck.rank_count(Rank::Nine)
    }

    pub(crate) fn hand_score(&self, hand: PokerHand) -> HandScore {
        self.hand_scores[hand as usize - 1]
    }

    pub(crate) fn upgrade_hand_score(&mut self, hand: PokerHand, chips: u16, mult: u16) {
        let score = &mut self.hand_scores[hand as usize - 1];
        score.chips = score.chips.saturating_add(chips);
        score.mult = score.mult.saturating_add(mult);
    }

    pub(crate) fn downgrade_hand_score(&mut self, hand: PokerHand) {
        let score = &mut self.hand_scores[hand as usize - 1];
        let (chips, mult) = crate::core::consumable::planet_upgrade_values(hand);
        score.chips = score.chips.saturating_sub(chips).max(1);
        score.mult = score.mult.saturating_sub(mult).max(1);
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
        if self.consumables.len() >= self.consumable_capacity() {
            return Err(ConsumableUseError::NoCapacity);
        }
        consumable.apply_discount(self.shop_discount_percent);
        self.consumables.push(consumable);
        Ok(())
    }

    pub(crate) fn add_consumable_during_use(
        &mut self,
        consumable: Consumable,
    ) -> Result<(), ConsumableUseError> {
        if self.consumables.len() >= self.consumable_capacity().saturating_add(1) {
            return Err(ConsumableUseError::NoCapacity);
        }
        let mut owned = OwnedConsumable::new(consumable, DEFAULT_CONSUMABLE_PRICE);
        owned.apply_discount(self.shop_discount_percent);
        self.consumables.push(owned);
        Ok(())
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
        owned.consumable.apply(self, hand, &target)?;

        let used = self.consumables.remove(index);
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
        Ok(())
    }

    pub(crate) fn apply_shop_discount_to_owned_items(&mut self) {
        for joker in self.jokers.as_mut_slice() {
            joker.apply_discount(self.shop_discount_percent);
        }
        for consumable in &mut self.consumables {
            consumable.apply_discount(self.shop_discount_percent);
        }
    }

    pub(crate) fn deal_cards(&mut self, hand: &mut Hand) {
        let hand_size = self
            .blind
            .draw_amount(self.hand_size().max(0) as usize)
            .min(u8::MAX as usize) as u8;
        self.deck.deal_cards(hand, hand_size, &mut self.rng);
        for card in hand.cards_mut() {
            self.blind.prepare_drawn_card(card, &mut self.rng);
        }
        self.blind
            .choose_forced_card(hand.hand_size() as usize, &mut self.rng);
    }

    pub(crate) fn discard_selected_cards(
        &mut self,
        hand: &mut Hand,
        positions: Vec<u8>,
    ) -> Option<Vec<Card>> {
        self.discard_selected_cards_with_action(hand, positions, true)
    }

    fn discard_selected_cards_with_action(
        &mut self,
        hand: &mut Hand,
        positions: Vec<u8>,
        counts_as_discard: bool,
    ) -> Option<Vec<Card>> {
        let discarded = hand.select_cards(positions);
        if discarded.is_empty() {
            return None;
        }

        self.deck.discard_cards(discarded.clone());
        self.update_jokers(UpdateEvent::CardsDiscarded { cards: &discarded });
        self.update_jokers(UpdateEvent::DiscardActionCompleted);
        if counts_as_discard {
            self.blind.record_discard();
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
        self.discard_selected_cards_with_action(
            hand,
            positions
                .into_iter()
                .map(|position| position as u8)
                .collect(),
            false,
        )
        .unwrap_or_default()
    }

    pub(crate) fn play_hand(
        &mut self,
        hand: &mut Hand,
        selected_positions: Vec<u8>,
    ) -> Option<ScoreResult> {
        let selected_indices = selected_positions
            .iter()
            .map(|&position| position as usize)
            .collect::<Vec<_>>();
        let score = crate::core::blind::calculate_score(self, hand, &selected_indices)?;
        let played_cards = hand.select_cards(selected_positions);
        if played_cards.is_empty() {
            return None;
        }
        self.deck.discard_cards(played_cards);
        self.apply_hook_discard(hand);
        self.deal_cards(hand);
        self.blind.chips_scored = self
            .blind
            .chips_scored
            .saturating_add(score.score.max(0.0) as u64);
        if self.blind.has_beaten_target() {
            self.finish_blind();
        }
        Some(score)
    }

    pub(crate) fn trigger_jokers(&mut self, event: TriggerEvent<'_>) -> JokerEffect {
        let mut jokers = std::mem::replace(&mut self.jokers, Jokers::new());
        let effect = jokers.trigger(event, self);
        self.jokers = jokers;
        effect
    }

    pub(crate) fn update_jokers(&mut self, event: UpdateEvent<'_>) -> JokerEffect {
        let mut jokers = std::mem::replace(&mut self.jokers, Jokers::new());
        let effect = jokers.update(event, self);
        self.jokers = jokers;
        effect
    }
}

/// The runtime object that will coordinate phase transitions.
pub struct Game {
    pub(crate) state: GameState,
    shop_active: bool,
}

impl Game {
    pub fn new(seed: u64) -> Game {
        Game {
            state: GameState::new(seed),
            shop_active: false,
        }
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
        Ok(())
    }

    pub(crate) fn shop(&self) -> Option<&ShopState> {
        self.shop_active.then_some(&self.state.shop)
    }

    pub(crate) fn shop_mut(&mut self) -> Option<&mut ShopState> {
        self.shop_active.then_some(&mut self.state.shop)
    }

    pub(crate) fn leave_shop(&mut self) -> Option<()> {
        self.shop_active.then_some(()).inspect(|_| {
            self.shop_active = false;
        })
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
