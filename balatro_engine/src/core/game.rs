use crate::core::blind::{BlindKind, BlindState};
use crate::core::consumable::{
    ConsumableTarget, ConsumableUseError, DEFAULT_CONSUMABLE_SLOTS, PokerHandLevel, PokerHandLevels,
};
use crate::core::deck::Deck;
use crate::core::enums::{Consumable, Decks, PokerHand, Rank, Tarot, Vouchers};
use crate::core::hand::Hand;
use crate::core::joker::{JokerEffect, Jokers, TriggerEvent, UpdateEvent};
use crate::core::pack::PackError;
use crate::core::shop::{ShopError, ShopState};
use crate::core::voucher::VoucherError;
use rand::SeedableRng;
use rand::prelude::IndexedRandom;
use rand::rngs::StdRng;

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
    pub(crate) hand_size_modifier: i8,
    pub(crate) hands_bonus: i8,
    pub(crate) discards_bonus: i8,
    pub(crate) consumable_slot_modifier: i8,
    pub(crate) joker_slot_modifier: i8,
    pub(crate) interest_cap: i16,
    pub(crate) shop_discount_percent: u8,
    pub(crate) reroll_minimum: u16,
    pub(crate) edition_rate_multiplier: u8,
    pub(crate) boss_rerolls_per_ante: u8,
    pub(crate) boss_reroll_unlimited: bool,
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
            hand_size_modifier: 0,
            hands_bonus: 0,
            discards_bonus: 0,
            consumable_slot_modifier: 0,
            joker_slot_modifier: 0,
            interest_cap: 5,
            shop_discount_percent: 0,
            reroll_minimum: 5,
            edition_rate_multiplier: 1,
            boss_rerolls_per_ante: 0,
            boss_reroll_unlimited: false,
        }
    }
}

pub struct GameState {
    pub(crate) blind: BlindState,
    pub(crate) deck: Deck,
    pub(crate) jokers: Jokers,
    pub(crate) money: i16,
    pub(crate) vouchers: Vec<Vouchers>,
    pub(crate) consumables: Vec<Consumable>,
    pub(crate) last_consumable: Option<Consumable>,
    pub(crate) hand_levels: PokerHandLevels,
    pub(crate) counters: GameCounters,
    pub(crate) most_played_hand: Option<PokerHand>,
    pub(crate) ante_voucher: Option<Vouchers>,
    pub(crate) rng: StdRng,
}

impl GameState {
    pub fn new(seed: u64) -> GameState {
        GameState::with_deck(seed, Decks::Red)
    }

    pub(crate) fn with_deck(seed: u64, deck_type: Decks) -> GameState {
        let mut rng = StdRng::seed_from_u64(seed);

        let mut state = GameState {
            blind: BlindState::new(),
            deck: Deck::new(deck_type, &mut rng),
            jokers: Jokers::new(),
            money: 0,
            vouchers: Vec::new(),
            consumables: Vec::new(),
            last_consumable: None,
            hand_levels: PokerHandLevels::new(),
            counters: GameCounters::new(),
            most_played_hand: None,
            ante_voucher: None,
            rng,
        };
        state.roll_ante_voucher();
        state
    }

    pub(crate) fn hand_size(&self) -> i8 {
        self.deck.hand_size() + self.counters.hand_size_modifier
    }

    pub(crate) fn consumable_capacity(&self) -> usize {
        (DEFAULT_CONSUMABLE_SLOTS as i8
            + self.deck.consumable_slot_modifier()
            + self.counters.consumable_slot_modifier)
            .max(0) as usize
    }

    pub(crate) fn joker_capacity(&self) -> usize {
        (Jokers::CAPACITY as i8 + self.counters.joker_slot_modifier).max(0) as usize
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
        (4i8 + self.counters.hands_bonus).max(0) as u8
    }

    pub(crate) fn discards_per_blind(&self) -> u8 {
        (4i8 + self.counters.discards_bonus).max(0) as u8
    }

    pub(crate) fn begin_blind(&mut self) {
        self.begin_blind_as(BlindKind::Small);
    }

    pub(crate) fn begin_blind_as(&mut self, kind: BlindKind) {
        self.blind.begin(
            kind,
            self.counters.ante,
            self.hands_per_blind(),
            self.discards_per_blind(),
        );
    }

    pub(crate) fn begin_next_ante(&mut self) {
        self.counters.ante = self.counters.ante.saturating_add(1);
        self.roll_ante_voucher();
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
                matches!(consumable, Consumable::Planet(planet) if crate::core::consumable::planet_hand(*planet) == hand)
            })
            .count() as i32;
        1.5_f32.powi(matching_planets)
    }

    pub(crate) fn shop_price(&self, base_price: u16) -> u16 {
        let remaining_percent = 100u32.saturating_sub(self.counters.shop_discount_percent as u32);
        if base_price == 0 {
            return 0;
        }
        ((base_price as u32 * remaining_percent) / 100).max(1) as u16
    }

    pub(crate) fn redeem_voucher(&mut self, voucher: Vouchers) -> Result<(), VoucherError> {
        crate::core::voucher::redeem(self, voucher)
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

    pub(crate) fn hand_level(&self, hand: PokerHand) -> PokerHandLevel {
        self.hand_levels.get(hand)
    }

    pub(crate) fn add_consumable(
        &mut self,
        consumable: Consumable,
    ) -> Result<(), ConsumableUseError> {
        if self.consumables.len() >= self.consumable_capacity() {
            return Err(ConsumableUseError::NoCapacity);
        }
        self.consumables.push(consumable);
        Ok(())
    }

    pub(crate) fn use_consumable(
        &mut self,
        index: usize,
        hand: &mut Hand,
        target: ConsumableTarget,
    ) -> Result<(), ConsumableUseError> {
        let consumable = self
            .consumables
            .get(index)
            .copied()
            .ok_or(ConsumableUseError::ConsumableNotFound)?;
        consumable.apply(self, hand, &target)?;

        let used = self.consumables.remove(index);
        match used {
            Consumable::Tarot(Tarot::Fool) => {}
            Consumable::Tarot(_) | Consumable::Planet(_) => {
                self.last_consumable = Some(used);
            }
            Consumable::Spectral(_) => {}
        }

        match used {
            Consumable::Tarot(_) => {
                self.counters.tarot_cards_used = self.counters.tarot_cards_used.saturating_add(1);
                self.update_jokers(UpdateEvent::TarotCardUsed);
            }
            Consumable::Planet(_) => {
                self.counters.planet_cards_used = self.counters.planet_cards_used.saturating_add(1);
                if let Consumable::Planet(planet) = used {
                    self.counters.used_planets |= planet.mask();
                }
                self.update_jokers(UpdateEvent::PlanetCardUsed);
            }
            Consumable::Spectral(_) => {}
        }
        Ok(())
    }

    pub(crate) fn deal_cards(&mut self, hand: &mut Hand) {
        let hand_size = self.hand_size().max(0) as u8;
        self.deck.deal_cards(hand, hand_size, &mut self.rng);
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
    pub(crate) shop: Option<ShopState>,
}

impl Game {
    pub fn new(seed: u64) -> Game {
        Game {
            state: GameState::new(seed),
            shop: None,
        }
    }

    pub(crate) fn enter_shop(&mut self, mut shop: ShopState) -> Result<(), ShopError> {
        if self.shop.is_some() {
            return Err(ShopError::ShopAlreadyActive);
        }
        shop.refresh_base_items(&mut self.state);
        shop.refresh_packs(&mut self.state);
        shop.refresh_voucher(&mut self.state);
        self.shop = Some(shop);
        Ok(())
    }

    pub(crate) fn shop(&self) -> Option<&ShopState> {
        self.shop.as_ref()
    }

    pub(crate) fn shop_mut(&mut self) -> Option<&mut ShopState> {
        self.shop.as_mut()
    }

    pub(crate) fn leave_shop(&mut self) -> Option<ShopState> {
        self.shop.take()
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
