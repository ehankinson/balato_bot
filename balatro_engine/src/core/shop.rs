use crate::core::card::Card;
use crate::core::consumable::DEFAULT_CONSUMABLE_SLOTS;
use crate::core::enums::{Consumable, Planet, Tarot, Vouchers};
use crate::core::game::GameState;
use crate::core::joker::{Joker, UpdateEvent};
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

    pub(crate) fn total(self) -> u32 {
        self.jokers + self.tarot + self.planets
    }

    pub(crate) fn kind_for_roll(self, roll: u32) -> ShopOfferKind {
        if roll < self.jokers {
            ShopOfferKind::Joker
        } else if roll < self.jokers + self.tarot {
            ShopOfferKind::Tarot
        } else {
            ShopOfferKind::Planet
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
    ShopAlreadyActive,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(crate) enum BoosterPackKind {
    Standard,
    Arcana,
    Celestial,
    Buffoon,
    Spectral,
}

#[derive(Clone)]
pub(crate) struct ShopVoucher {
    pub(crate) voucher: Vouchers,
    pub(crate) price: u16,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(crate) struct ShopPack {
    pub(crate) kind: BoosterPackKind,
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
    pub(crate) reroll_cost: u16,
}

impl ShopState {
    pub(crate) fn new() -> ShopState {
        ShopState {
            voucher: None,
            packs: Vec::with_capacity(SHOP_PACK_SLOTS),
            items: Vec::new(),
            reroll_cost: DEFAULT_REROLL_COST,
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

    pub(crate) fn add_pack(&mut self, pack: ShopPack) -> Result<(), ShopError> {
        if self.packs.len() >= SHOP_PACK_SLOTS {
            return Err(ShopError::InvalidOffer);
        }
        self.packs.push(pack);
        Ok(())
    }

    pub(crate) fn add_item(&mut self, item: ShopItem) {
        self.items.push(item);
    }

    pub(crate) fn base_item_slots(state: &GameState) -> usize {
        2 + usize::from(state.vouchers.contains(&Vouchers::Overstock))
            + usize::from(state.vouchers.contains(&Vouchers::OverstockPlus))
    }

    pub(crate) fn refresh_base_items(&mut self, state: &mut GameState) {
        let item_slots = Self::base_item_slots(state);
        self.items.clear();
        self.items.reserve(item_slots);

        for _ in 0..item_slots {
            let kind = Self::roll_base_offer_kind(state);
            self.items.push(random_base_item(kind, state));
        }
    }

    pub(crate) fn roll_base_offer_kind(state: &mut GameState) -> ShopOfferKind {
        ShopOfferWeights::BASE.roll(&mut state.rng)
    }

    pub(crate) fn buy_item(
        &mut self,
        index: usize,
        state: &mut GameState,
    ) -> Result<(), ShopError> {
        let item = self.items.get(index).ok_or(ShopError::InvalidOffer)?;
        let price = item.price();
        require_funds(state, price)?;

        match item {
            ShopItem::Joker { .. } if !state.jokers.has_room() => {
                return Err(ShopError::InventoryFull);
            }
            ShopItem::Consumable { .. } if state.consumables.len() >= DEFAULT_CONSUMABLE_SLOTS => {
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
                state.counters.empty_joker_slots =
                    state.counters.empty_joker_slots.saturating_sub(1);
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

    pub(crate) fn buy_voucher(&mut self, state: &mut GameState) -> Result<(), ShopError> {
        let offer = self.voucher.as_ref().ok_or(ShopError::InvalidOffer)?;
        if state.vouchers.contains(&offer.voucher) {
            return Err(ShopError::VoucherAlreadyOwned);
        }
        require_funds(state, offer.price)?;

        let offer = self.voucher.take().expect("checked above");
        state.money -= offer.price as i16;
        state.vouchers.push(offer.voucher);
        Ok(())
    }

    pub(crate) fn buy_pack(
        &mut self,
        index: usize,
        state: &mut GameState,
    ) -> Result<ShopPack, ShopError> {
        let pack = self.packs.get(index).ok_or(ShopError::InvalidOffer)?;
        require_funds(state, pack.price)?;

        let pack = self.packs.remove(index);
        state.money -= pack.price as i16;
        Ok(pack)
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
            let joker = Joker::random_shop_joker(&mut state.rng);
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
            let planet = Planet::ALL
                .choose(&mut state.rng)
                .copied()
                .expect("the Planet table must not be empty");
            ShopItem::Consumable {
                consumable: Consumable::Planet(planet),
                price: DEFAULT_CONSUMABLE_PRICE,
            }
        }
    }
}

fn require_funds(state: &GameState, price: u16) -> Result<(), ShopError> {
    if state.money >= price as i16 {
        Ok(())
    } else {
        Err(ShopError::InsufficientFunds)
    }
}
