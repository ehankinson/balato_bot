use crate::core::card::Card;
use crate::core::enums::{Consumable, Decks, Planet, Spectral, Tarot, Vouchers};
use crate::core::game::GameState;
use crate::core::joker::{Joker, UpdateEvent};
use crate::core::pack::{
    PackOpening, ShopPack, planet_for_hand, random_shop_pack, random_shop_playing_card,
};
use rand::prelude::{IndexedRandom, RngExt};
use rand::rngs::StdRng;

pub(crate) const DEFAULT_REROLL_COST: u16 = 5;
pub(crate) const DEFAULT_CONSUMABLE_PRICE: u16 = 3;
pub(crate) const SHOP_PACK_SLOTS: usize = 2;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(crate) enum ShopOfferKind {
    Joker,
    Tarot,
    Planet,
    Spectral,
    PlayingCard,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(crate) struct ShopOfferWeights {
    pub(crate) jokers: u32,
    pub(crate) tarot: u32,
    pub(crate) planets: u32,
    pub(crate) spectrals: u32,
    pub(crate) cards: u32,
}

impl ShopOfferWeights {
    pub(crate) const BASE: ShopOfferWeights = ShopOfferWeights {
        jokers: 20,
        tarot: 4,
        planets: 4,
        spectrals: 0,
        cards: 0,
    };

    pub(crate) fn multiply_tarot_weight(&mut self, multiplier: u32) {
        self.tarot = self.tarot.saturating_mul(multiplier);
    }

    pub(crate) fn multiply_planet_weight(&mut self, multiplier: u32) {
        self.planets = self.planets.saturating_mul(multiplier);
    }

    pub(crate) fn total(self) -> u32 {
        self.jokers + self.tarot + self.planets + self.spectrals + self.cards
    }

    pub(crate) fn kind_for_roll(self, roll: u32) -> ShopOfferKind {
        if roll < self.jokers {
            ShopOfferKind::Joker
        } else if roll < self.jokers + self.tarot {
            ShopOfferKind::Tarot
        } else if roll < self.jokers + self.tarot + self.planets {
            ShopOfferKind::Planet
        } else if roll < self.jokers + self.tarot + self.planets + self.spectrals {
            ShopOfferKind::Spectral
        } else {
            ShopOfferKind::PlayingCard
        }
    }

    pub(crate) fn roll(self, rng: &mut StdRng) -> ShopOfferKind {
        self.kind_for_roll(rng.random_range(0..self.total()))
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ShopError {
    InvalidOffer,
    InsufficientFunds,
    InventoryFull,
    VoucherAlreadyOwned,
    VoucherPrerequisiteMissing,
    ShopAlreadyActive,
}

#[derive(Clone)]
pub(crate) struct ShopVoucher {
    pub(crate) voucher: Vouchers,
    pub(crate) price: u16,
}

#[derive(Clone)]
pub(crate) enum ShopItem {
    Joker { joker: Joker, price: u16 },
    Consumable { consumable: Consumable, price: u16 },
    PlayingCard { card: Card, price: u16 },
}

impl ShopItem {
    pub(crate) fn price(&self) -> u16 {
        match self {
            ShopItem::Joker { price, .. }
            | ShopItem::Consumable { price, .. }
            | ShopItem::PlayingCard { price, .. } => *price,
        }
    }
}

pub(crate) struct ShopState {
    pub(crate) voucher: Option<ShopVoucher>,
    pub(crate) packs: Vec<ShopPack>,
    pub(crate) items: Vec<ShopItem>,
    pub(crate) number_of_shop_items: usize,
    pub(crate) initial_reroll_cost: u16,
    pub(crate) reroll_cost: u16,
    pub(crate) offer_weights: ShopOfferWeights,
}

impl ShopState {
    pub(crate) fn new() -> ShopState {
        ShopState {
            voucher: None,
            packs: Vec::with_capacity(SHOP_PACK_SLOTS),
            items: Vec::new(),
            number_of_shop_items: 2,
            initial_reroll_cost: DEFAULT_REROLL_COST,
            reroll_cost: DEFAULT_REROLL_COST,
            offer_weights: ShopOfferWeights::BASE,
        }
    }

    pub(crate) fn with_reroll_cost(reroll_cost: u16) -> ShopState {
        ShopState {
            reroll_cost,
            ..ShopState::new()
        }
    }

    pub(crate) fn set_voucher(&mut self, voucher: ShopVoucher) {
        self.voucher = Some(voucher);
    }

    pub(crate) fn begin_shop(&mut self) {
        self.voucher = None;
        self.packs.clear();
        self.items.clear();
        self.reroll_cost = self.initial_reroll_cost;
    }

    pub(crate) fn apply_deck_rules(&mut self, deck_type: Decks) {
        if deck_type == Decks::Ghost {
            self.offer_weights.spectrals = 2;
        }
    }

    pub(crate) fn refresh_voucher(&mut self, state: &mut GameState) {
        if self.voucher.is_some() {
            return;
        }

        self.voucher = state
            .ante_voucher
            .filter(|voucher| state.can_redeem_voucher(*voucher))
            .map(|voucher| ShopVoucher { voucher, price: 10 });
    }

    pub(crate) fn add_pack(&mut self, pack: ShopPack) -> Result<(), ShopError> {
        if self.packs.len() >= SHOP_PACK_SLOTS {
            return Err(ShopError::InvalidOffer);
        }
        self.packs.push(pack);
        Ok(())
    }

    pub(crate) fn refresh_packs(&mut self, state: &mut GameState) {
        self.packs.clear();
        for _ in 0..SHOP_PACK_SLOTS {
            self.packs.push(random_shop_pack(state));
        }
    }

    pub(crate) fn add_item(&mut self, item: ShopItem) {
        self.items.push(item);
    }

    pub(crate) fn refresh_base_items(&mut self, state: &mut GameState) {
        self.items.clear();
        self.items.reserve(self.number_of_shop_items);

        self.refill_base_items(state);
    }

    pub(crate) fn refill_base_items(&mut self, state: &mut GameState) {
        self.items
            .reserve(self.number_of_shop_items.saturating_sub(self.items.len()));
        while self.items.len() < self.number_of_shop_items {
            let kind = self.roll_base_offer_kind(state);
            self.items.push(random_base_item(kind, state));
        }
    }

    pub(crate) fn roll_base_offer_kind(&self, state: &mut GameState) -> ShopOfferKind {
        self.offer_weights.roll(&mut state.rng)
    }

    pub(crate) fn buy_item(
        &mut self,
        index: usize,
        state: &mut GameState,
    ) -> Result<(), ShopError> {
        let item = self.items.get(index).ok_or(ShopError::InvalidOffer)?;
        let price = state.shop_price(item.price());
        require_funds(state, price)?;

        match item {
            ShopItem::Joker { .. } if !state.joker_has_room() => {
                return Err(ShopError::InventoryFull);
            }
            ShopItem::Consumable { .. }
                if state.consumables.len() >= state.consumable_capacity() =>
            {
                return Err(ShopError::InventoryFull);
            }
            ShopItem::Joker { .. } | ShopItem::Consumable { .. } | ShopItem::PlayingCard { .. } => {
            }
        }

        let item = self.items.remove(index);
        state.money -= price as i16;
        match item {
            ShopItem::Joker { joker, .. } => {
                state.jokers.add(joker);
                state.update_jokers(UpdateEvent::JokerOrderChanged);
            }
            ShopItem::Consumable { consumable, .. } => {
                state.consumables.push(consumable);
            }
            ShopItem::PlayingCard { card, .. } => {
                state.deck.add_card_to_deck(card);
                state.update_jokers(UpdateEvent::DeckChanged { cards_added: 1 });
            }
        }
        Ok(())
    }

    pub(crate) fn buy_pack(
        &mut self,
        index: usize,
        state: &mut GameState,
    ) -> Result<ShopPack, ShopError> {
        let pack = self.packs.get(index).ok_or(ShopError::InvalidOffer)?;
        let price = state.shop_price(pack.price);
        require_funds(state, price)?;

        let pack = self.packs.remove(index);
        state.money -= price as i16;
        Ok(pack)
    }

    pub(crate) fn buy_and_open_pack(
        &mut self,
        index: usize,
        state: &mut GameState,
    ) -> Result<PackOpening, ShopError> {
        let pack = self.packs.get(index).ok_or(ShopError::InvalidOffer)?;
        let price = state.shop_price(pack.price);
        require_funds(state, price)?;

        let pack = self.packs.remove(index);
        state.money -= price as i16;
        Ok(pack.open(state))
    }

    pub(crate) fn reroll(&mut self, state: &mut GameState) -> Result<(), ShopError> {
        require_funds(state, self.reroll_cost)?;
        state.money -= self.reroll_cost as i16;
        state.counters.shop_rerolls = state.counters.shop_rerolls.saturating_add(1);

        let next_reroll_cost = self.reroll_cost.saturating_add(1);
        state.update_jokers(UpdateEvent::ShopRerolled);
        self.refresh_base_items(state);
        self.reroll_cost = next_reroll_cost;
        Ok(())
    }
}

fn random_base_item(kind: ShopOfferKind, state: &mut GameState) -> ShopItem {
    match kind {
        ShopOfferKind::Joker => {
            let mut joker = Joker::random_shop_joker(&mut state.rng);
            joker.set_edition(crate::core::voucher::random_shop_joker_edition(state));
            let price = joker.price();
            ShopItem::Joker { joker, price }
        }
        ShopOfferKind::Tarot => {
            let tarot = Tarot::ALL
                .choose(&mut state.rng)
                .copied()
                .expect("the Tarot table must not be empty");
            ShopItem::Consumable {
                consumable: Consumable::Tarot(tarot),
                price: DEFAULT_CONSUMABLE_PRICE,
            }
        }
        ShopOfferKind::Planet => {
            let planet = state
                .most_played_hand
                .filter(|_| state.has_voucher(Vouchers::Telescope))
                .map(planet_for_hand)
                .unwrap_or_else(|| {
                    Planet::ALL
                        .choose(&mut state.rng)
                        .copied()
                        .expect("the Planet table must not be empty")
                });
            ShopItem::Consumable {
                consumable: Consumable::Planet(planet),
                price: DEFAULT_CONSUMABLE_PRICE,
            }
        }
        ShopOfferKind::Spectral => {
            let spectral = Spectral::ALL
                .choose(&mut state.rng)
                .copied()
                .expect("the Spectral table must not be empty");
            ShopItem::Consumable {
                consumable: Consumable::Spectral(spectral),
                price: DEFAULT_CONSUMABLE_PRICE,
            }
        }
        ShopOfferKind::PlayingCard => ShopItem::PlayingCard {
            card: random_shop_playing_card(state),
            price: 1,
        },
    }
}

fn require_funds(state: &GameState, price: u16) -> Result<(), ShopError> {
    if state.money >= price as i16 {
        Ok(())
    } else {
        Err(ShopError::InsufficientFunds)
    }
}
