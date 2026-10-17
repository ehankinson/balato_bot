//! Independently owned numeric observations of a decision boundary. Encoding
//! borrows Game immutably and never advances automatic work or consumes RNG.
use crate::core::blind::BlindKind;
use crate::core::card::{Card, CardProperties};
use crate::core::consumable::OwnedConsumable;
use crate::core::enums::{BossBlinds, SkipTag};
use crate::core::game::{BOSS_REROLL_COST, DecisionPoint, Game};
use crate::core::joker::{Joker, JokerValueKey};
use crate::core::joker_types::JokerEdition;
use crate::core::pack::PackOption;
use crate::core::shop::ShopItem;

#[path = "observation_ids.rs"]
mod ids;

pub(crate) const SCHEMA_VERSION: i32 = 1;

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ObservationError {
    UnsupportedPhase,
    InvalidState(&'static str),
    CollectionTooLarge,
    IntegerOverflow,
    NonFinite,
    InvalidProbability,
    AllocationFailed,
}

/// Rows of fixed-width arrays are contiguous; visibility is flattened in
/// categorical, numeric, flag order. Presence supplies the row count even for
/// zero-column groups. Single-game tables are unpadded.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Table<const C: usize, const N: usize, const F: usize> {
    pub(crate) cat: Vec<[i32; C]>,
    pub(crate) num: Vec<[f32; N]>,
    pub(crate) flags: Vec<[bool; F]>,
    pub(crate) present: Vec<bool>,
    pub(crate) visible: Vec<bool>,
}

struct Row<const C: usize, const N: usize, const F: usize> {
    cat: [i32; C],
    num: [f32; N],
    flags: [bool; F],
    cat_visible: [bool; C],
    num_visible: [bool; N],
    flags_visible: [bool; F],
    present: bool,
}

impl<const C: usize, const N: usize, const F: usize> Default for Row<C, N, F> {
    fn default() -> Self {
        Self {
            cat: [0; C],
            num: [0.0; N],
            flags: [false; F],
            cat_visible: [false; C],
            num_visible: [false; N],
            flags_visible: [false; F],
            present: false,
        }
    }
}

impl<const C: usize, const N: usize, const F: usize> Default for Table<C, N, F> {
    fn default() -> Self {
        Self {
            cat: vec![],
            num: vec![],
            flags: vec![],
            present: vec![],
            visible: vec![],
        }
    }
}

impl<const C: usize, const N: usize, const F: usize> Table<C, N, F> {
    pub(crate) fn len(&self) -> usize {
        self.present.len()
    }

    fn push(&mut self, row: Row<C, N, F>) -> Result<(), ObservationError> {
        check_length(
            self.len()
                .checked_add(1)
                .ok_or(ObservationError::CollectionTooLarge)?,
        )?;
        reserve(&mut self.cat, 1)?;
        reserve(&mut self.num, 1)?;
        reserve(&mut self.flags, 1)?;
        reserve(&mut self.present, 1)?;
        reserve(&mut self.visible, C + N + F)?;
        self.cat.push(row.cat);
        self.num.push(row.num);
        self.flags.push(row.flags);
        self.present.push(row.present);
        self.visible.extend(row.cat_visible);
        self.visible.extend(row.num_visible);
        self.visible.extend(row.flags_visible);
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RunObservation {
    pub(crate) cat: [i32; 5],
    pub(crate) counts: [i64; 14],
    pub(crate) num: [f32; 3],
    pub(crate) extra: [i64; 3],
    pub(crate) used_planets: [bool; 12],
    pub(crate) flags: [bool; 3],
    pub(crate) visible: [bool; 28],
}

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct BlindObservation {
    pub(crate) cat: [i32; 4],
    pub(crate) counts: [i64; 5],
    pub(crate) num: [f32; 2],
    pub(crate) flags: [bool; 3],
    pub(crate) visible: [bool; 14],
    pub(crate) played_types: [bool; 12],
}

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct ShopObservation {
    pub(crate) num: [f32; 1],
    pub(crate) visible: [bool; 1],
}

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct PackObservation {
    pub(crate) cat: [i32; 3],
    pub(crate) counts: [i64; 1],
    pub(crate) visible: [bool; 4],
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Observation {
    pub(crate) schema_version: [i32; 1],
    pub(crate) sections: [bool; 5],
    pub(crate) run: RunObservation,
    pub(crate) jokers: Table<3, 1, 5>,
    pub(crate) joker_values: Table<3, 1, 0>,
    pub(crate) consumables: Table<2, 1, 1>,
    pub(crate) last_consumable: Table<2, 1, 1>,
    pub(crate) deck_catalogue: Table<5, 3, 0>,
    pub(crate) deck_multiplicity: Vec<i64>,
    pub(crate) poker_hands: Table<1, 2, 0>,
    pub(crate) poker_hand_counts: Vec<[i64; 2]>,
    pub(crate) vouchers: Table<1, 0, 0>,
    pub(crate) tags: Table<2, 1, 0>,
    pub(crate) blind_offers: Table<3, 2, 2>,
    pub(crate) blind: BlindObservation,
    pub(crate) hand: Table<5, 3, 3>,
    pub(crate) shop: ShopObservation,
    pub(crate) shop_items: Table<10, 5, 5>,
    pub(crate) shop_vouchers: Table<1, 1, 0>,
    pub(crate) shop_packs: Table<2, 1, 0>,
    pub(crate) pack: PackObservation,
    pub(crate) pack_options: Table<10, 5, 5>,
    pub(crate) action_kinds: [bool; 18],
}

impl Game {
    pub(crate) fn observe(&self) -> Result<Observation, ObservationError> {
        if self.decision_point() == DecisionPoint::ConsumableSelection {
            return Err(ObservationError::UnsupportedPhase);
        }
        let playing = self.decision_point() == DecisionPoint::PlayingHand;
        let selection = self.decision_point() == DecisionPoint::BlindSelection;
        let shop = self.decision_point() == DecisionPoint::Shop;
        let pack = self.decision_point() == DecisionPoint::PackSelection;
        if shop && self.shop().is_none() {
            return Err(ObservationError::InvalidState("inactive shop"));
        }
        if pack && self.pending_pack().is_none() {
            return Err(ObservationError::InvalidState("missing pack"));
        }
        let mut observation = Observation {
            schema_version: [SCHEMA_VERSION],
            sections: [selection, playing, shop, pack, self.has_active_hand()],
            run: encode_run(self)?,
            jokers: Table::default(),
            joker_values: Table::default(),
            consumables: Table::default(),
            last_consumable: Table::default(),
            deck_catalogue: Table::default(),
            deck_multiplicity: vec![],
            poker_hands: Table::default(),
            poker_hand_counts: vec![],
            vouchers: Table::default(),
            tags: Table::default(),
            blind_offers: Table::default(),
            blind: BlindObservation::default(),
            hand: Table::default(),
            shop: ShopObservation::default(),
            shop_items: Table::default(),
            shop_vouchers: Table::default(),
            shop_packs: Table::default(),
            pack: PackObservation::default(),
            pack_options: Table::default(),
            action_kinds: self.action_kinds(),
        };
        encode_persistent(self, &mut observation)?;
        encode_deck(self, &mut observation)?;
        encode_blind_offers(self, &mut observation)?;
        if playing {
            observation.blind = encode_blind(self)?;
        }
        if let Some(hand) = self.active_hand() {
            check_length(hand.cards().len())?;
            for (index, card) in hand.cards().iter().enumerate() {
                let forced = playing
                    && !self.state.blind.boss_ability_disabled
                    && matches!(
                        self.state.blind.kind,
                        BlindKind::Boss(BossBlinds::CeruleanBell)
                    )
                    && self.state.blind.forced_card_index == Some(index);
                observation.hand.push(hand_row(card, forced)?)?;
            }
        }
        if shop {
            encode_shop(self, &mut observation)?;
        }
        if pack {
            encode_pack(self, &mut observation)?;
        }
        Ok(observation)
    }
}

fn encode_run(game: &Game) -> Result<RunObservation, ObservationError> {
    let state = &game.state;
    let selection = game.decision_point() == DecisionPoint::BlindSelection;
    let draw = state.deck.cards().len();
    let held = game.hand().cards().len();
    let discarded = state.deck.discarded_cards().len();
    for length in [
        draw,
        held,
        discarded,
        state.jokers.as_slice().len(),
        state.consumables.len(),
    ] {
        check_length(length)?;
    }
    let total = draw
        .checked_add(held)
        .and_then(|count| count.checked_add(discarded))
        .ok_or(ObservationError::IntegerOverflow)?;
    let hand_size = i64::from(state.deck.hand_size())
        + state.jokers.hand_size_modifier()
        + if game.decision_point() == DecisionPoint::PlayingHand {
            i64::from(state.blind.hand_size_modifier)
        } else {
            0
        };
    let mut result = RunObservation {
        cat: [
            i32::from(game.decision_point().code()),
            ids::decks(state.deck.deck_type()),
            ids::stakes(state.blind.stake),
            if selection {
                blind_ids(game.available_blind())[0]
            } else {
                -1
            },
            i32::from(game.decision_point() == DecisionPoint::RunComplete),
        ],
        counts: [
            i64::from(state.counters.ante),
            i64::from(state.counters.round),
            hand_size.max(0),
            integer(state.joker_capacity())?,
            integer(state.consumable_capacity())?,
            integer(state.jokers.as_slice().len())?,
            integer(state.consumables.len())?,
            integer(total)?,
            integer(draw)?,
            integer(held)?,
            integer(discarded)?,
            i64::from(state.counters.tarot_cards_used),
            i64::from(state.counters.planet_cards_used),
            i64::from(state.counters.blinds_skipped),
        ],
        num: [
            log_number(f64::from(state.money))?,
            log_number(f64::from(state.interest_cap))?,
            0.0,
        ],
        extra: [
            i64::from(state.counters.cards_sold),
            i64::from(state.counters.shop_rerolls),
            i64::from(
                u32::from(state.boss_rerolls_per_ante)
                    .saturating_sub(state.counters.boss_rerolls_used),
            ),
        ],
        used_planets: std::array::from_fn(|index| state.counters.used_planets & (1 << index) != 0),
        flags: [
            game.decision_point() == DecisionPoint::RunComplete,
            state.counters.ante_eight_cleared,
            state.boss_reroll_unlimited,
        ],
        visible: [true; 28],
    };
    result.visible[3] = selection;
    let cost_visible =
        selection && (state.boss_rerolls_per_ante > 0 || state.boss_reroll_unlimited);
    result.visible[21] = cost_visible;
    if cost_visible {
        result.num[2] = log_number(f64::from(BOSS_REROLL_COST))?;
    }
    Ok(result)
}

fn encode_persistent(game: &Game, observation: &mut Observation) -> Result<(), ObservationError> {
    let state = &game.state;
    check_length(state.jokers.as_slice().len())?;
    for (index, joker) in state.jokers.as_slice().iter().enumerate() {
        let mut row = Row::<3, 1, 5> {
            present: true,
            ..Row::default()
        };
        row.flags[0] = joker.is_face_down();
        row.flags_visible[0] = true;
        if !joker.is_face_down() {
            row.cat = [
                ids::joker_kind(joker.kind()),
                ids::joker_rarity(joker.rarity()),
                ids::joker_edition(joker.edition()),
            ];
            row.cat_visible.fill(true);
            row.num = [log_number(f64::from(joker.sell_value()))?];
            row.num_visible.fill(true);
            row.flags = [
                false,
                joker.is_debuffed(),
                joker.is_eternal(),
                joker.perishable_rounds().is_some(),
                joker.is_rental(),
            ];
            row.flags_visible.fill(true);
        }
        observation.jokers.push(row)?;
        encode_joker_values(joker, &state.blind, 0, index, &mut observation.joker_values)?;
    }
    check_length(state.consumables.len())?;
    for owned in &state.consumables {
        observation.consumables.push(consumable_row(owned)?)?;
    }
    if let Some(owned) = &state.last_consumable {
        observation.last_consumable.push(consumable_row(owned)?)?;
    }
    if state.hand_scores.len() != 12 {
        return Err(ObservationError::InvalidState("poker-hand table length"));
    }
    for (index, score) in state.hand_scores.iter().enumerate() {
        observation.poker_hands.push(Row {
            cat: [index as i32 + 1],
            num: [
                log_number(f64::from(score.chips))?,
                log_number(f64::from(score.mult))?,
            ],
            cat_visible: [true],
            num_visible: [true; 2],
            present: true,
            ..Row::default()
        })?;
        reserve(&mut observation.poker_hand_counts, 1)?;
        observation.poker_hand_counts.push([
            integer(score.level)?,
            i64::from(state.counters.poker_hands_played[index]),
        ]);
    }
    check_length(state.vouchers.len())?;
    for voucher in &state.vouchers {
        observation.vouchers.push(Row {
            cat: [ids::vouchers(*voucher)],
            cat_visible: [true],
            present: true,
            ..Row::default()
        })?;
    }
    check_length(state.tags.pending().len())?;
    for tag in state.tags.pending() {
        let value = match tag {
            SkipTag::Investment => Some(25.0),
            SkipTag::Juggle => Some(3.0),
            SkipTag::Double | SkipTag::Voucher => Some(1.0),
            _ => None,
        };
        let mut row = Row {
            cat: [ids::skip_tag(*tag), 0],
            cat_visible: [true, false],
            present: true,
            ..Row::default()
        };
        if let Some(value) = value {
            row.num[0] = log_number(value)?;
            row.num_visible[0] = true;
        }
        observation.tags.push(row)?;
    }
    Ok(())
}

fn consumable_row(owned: &OwnedConsumable) -> Result<Row<2, 1, 1>, ObservationError> {
    Ok(Row {
        cat: ids::consumable(owned.consumable),
        num: [log_number(f64::from(owned.sell_value()))?],
        flags: [owned.negative],
        cat_visible: [true; 2],
        num_visible: [true],
        flags_visible: [true],
        present: true,
    })
}

fn encode_joker_values(
    joker: &Joker,
    blind: &crate::core::blind::BlindState,
    parent: i32,
    index: usize,
    values: &mut Table<3, 1, 0>,
) -> Result<(), ObservationError> {
    for value in joker.visible_values(blind) {
        if value.key == JokerValueKey::Probability && !(0.0..=1.0).contains(&value.value) {
            return Err(if value.value.is_finite() {
                ObservationError::InvalidProbability
            } else {
                ObservationError::NonFinite
            });
        }
        let number = if value.key.is_raw() {
            raw_number(value.value)?
        } else {
            log_number(value.value)?
        };
        values.push(Row {
            cat: [
                parent,
                i32::try_from(index).map_err(|_| ObservationError::CollectionTooLarge)?,
                value.key as i32,
            ],
            num: [number],
            cat_visible: [true; 3],
            num_visible: [true],
            present: true,
            ..Row::default()
        })?;
    }
    Ok(())
}

fn card_categories(properties: &CardProperties) -> [i32; 5] {
    [
        ids::rank(properties.rank),
        ids::suit(properties.suit),
        ids::enhancement(properties.enhancement),
        ids::edition(properties.edition),
        ids::seal(properties.seal),
    ]
}

fn hand_row(card: &Card, forced: bool) -> Result<Row<5, 3, 3>, ObservationError> {
    let mut row = Row {
        flags: [card.is_face_down(), false, forced],
        flags_visible: [true, false, true],
        present: true,
        ..Row::default()
    };
    if !card.is_face_down() {
        row.cat = card_categories(&card.inspectable_properties());
        row.cat_visible.fill(true);
        let score = card.scoring_values();
        row.num = [
            log_number(f64::from(score.chips))?,
            log_number(f64::from(score.add_mult))?,
            log_number(f64::from(score.x_mult))?,
        ];
        row.num_visible.fill(true);
        row.flags[1] = card.is_debuffed();
        row.flags_visible[1] = true;
    }
    Ok(row)
}

fn encode_deck(game: &Game, observation: &mut Observation) -> Result<(), ObservationError> {
    let mut entries = Vec::new();
    let uncertain = game.state.deck.composition_uncertain();
    let mut unknown = 0i64;
    for (inspectable, card) in game
        .state
        .deck
        .cards()
        .iter()
        .map(|card| (!uncertain, card))
        .chain(
            game.hand()
                .cards()
                .iter()
                .map(|card| (!uncertain || !card.is_face_down(), card)),
        )
        .chain(
            game.state
                .deck
                .discarded_cards()
                .iter()
                .map(|card| (!uncertain || !card.is_face_down(), card)),
        )
    {
        // An anonymous concealed removal cannot reveal the removed identity via
        // a before/after histogram. Preserve guaranteed public occurrences and
        // represent the rest with an explicitly unknown multiset row.
        if !inspectable {
            unknown += 1;
            continue;
        }
        check_length(
            entries
                .len()
                .checked_add(1)
                .ok_or(ObservationError::CollectionTooLarge)?,
        )?;
        let properties = card.inspectable_properties();
        let raw = [
            f64::from(properties.bonuses.chips),
            f64::from(properties.bonuses.add_mult),
            f64::from(properties.bonuses.x_mult),
        ];
        let num = [
            log_number(raw[0])?,
            log_number(raw[1])?,
            log_number(raw[2])?,
        ];
        reserve(&mut entries, 1)?;
        entries.push((
            card_categories(&properties),
            raw.map(|value| if value == 0.0 { 0.0 } else { value }),
            num,
        ));
    }
    entries.sort_unstable_by(|left, right| {
        left.0
            .cmp(&right.0)
            .then_with(|| left.1[0].total_cmp(&right.1[0]))
            .then_with(|| left.1[1].total_cmp(&right.1[1]))
            .then_with(|| left.1[2].total_cmp(&right.1[2]))
    });
    if unknown > 0 {
        observation.deck_catalogue.push(Row {
            present: true,
            ..Row::default()
        })?;
        reserve(&mut observation.deck_multiplicity, 1)?;
        observation.deck_multiplicity.push(unknown);
    }
    let mut previous = None;
    for (cat, raw, num) in entries {
        if previous == Some((cat, raw)) {
            *observation.deck_multiplicity.last_mut().unwrap() += 1;
        } else {
            observation.deck_catalogue.push(Row {
                cat,
                num,
                cat_visible: [true; 5],
                num_visible: [true; 3],
                present: true,
                ..Row::default()
            })?;
            reserve(&mut observation.deck_multiplicity, 1)?;
            observation.deck_multiplicity.push(1);
            previous = Some((cat, raw));
        }
    }
    Ok(())
}

fn blind_ids(kind: BlindKind) -> [i32; 2] {
    match kind {
        BlindKind::Small => [0, 0],
        BlindKind::Big => [1, 0],
        BlindKind::Boss(boss) => [2, ids::boss_blinds(boss)],
    }
}

fn encode_blind_offers(game: &Game, observation: &mut Observation) -> Result<(), ObservationError> {
    let state = &game.state;
    let selector = state.blind_selector;
    for offer in [selector.small(), selector.big(), selector.boss()] {
        let mut row = Row::<3, 2, 2>::default();
        if game.decision_point() == DecisionPoint::BlindSelection {
            let [kind, boss] = blind_ids(offer.kind);
            row.cat = [kind, boss, offer.skip_tag.map_or(0, ids::skip_tag)];
            row.cat_visible = [true, offer.kind.is_boss(), offer.skip_tag.is_some()];
            row.num = [
                log_number(
                    offer
                        .kind
                        .score_requirement(state.blind.stake, state.counters.ante, false)
                        .saturating_mul(state.blind.ante_scaling) as f64,
                )?,
                log_number(f64::from(offer.kind.reward(state.blind.stake)))?,
            ];
            row.num_visible.fill(true);
            row.flags_visible.fill(true);
            row.present = true;
            row.flags = [
                offer.kind == game.available_blind(),
                offer.kind == game.available_blind() && !offer.kind.is_boss(),
            ];
        }
        observation.blind_offers.push(row)?;
    }
    Ok(())
}

fn encode_blind(game: &Game) -> Result<BlindObservation, ObservationError> {
    let blind = &game.state.blind;
    let [kind, boss] = blind_ids(blind.kind);
    let ox = matches!(blind.kind, BlindKind::Boss(BossBlinds::Ox));
    let first = blind.played_hand_types.first().copied();
    let forced = if !blind.boss_ability_disabled
        && matches!(blind.kind, BlindKind::Boss(BossBlinds::CeruleanBell))
    {
        blind.forced_card_index
    } else {
        None
    };
    if forced.is_some_and(|index| index >= game.hand().cards().len()) {
        return Err(ObservationError::InvalidState("forced hand index"));
    }
    let mut result = BlindObservation {
        cat: [
            kind,
            boss,
            if ox {
                ids::poker_hand(game.state.ox_target_hand)
            } else {
                0
            },
            first.map_or(0, ids::poker_hand),
        ],
        counts: [
            i64::from(blind.hands_remaining),
            i64::from(blind.discards_remaining),
            i64::from(blind.hands_played),
            i64::from(blind.discards_used),
            forced.map(integer).transpose()?.unwrap_or(-1),
        ],
        num: [
            log_number(blind.chips_scored as f64)?,
            log_number(blind.score_requirement as f64)?,
        ],
        flags: [
            blind.boss_ability_disabled,
            blind.first_action_taken,
            blind.finished,
        ],
        visible: [true; 14],
        played_types: [false; 12],
    };
    result.visible[1] = blind.kind.is_boss();
    result.visible[2] = ox;
    result.visible[3] = first.is_some();
    result.visible[8] = forced.is_some();
    for hand in &blind.played_hand_types {
        result.played_types[(ids::poker_hand(*hand) - 1) as usize] = true;
    }
    Ok(result)
}

fn encode_shop(game: &Game, observation: &mut Observation) -> Result<(), ObservationError> {
    let shop = game
        .shop()
        .ok_or(ObservationError::InvalidState("inactive shop"))?;
    observation.shop = ShopObservation {
        num: [log_number(f64::from(shop.reroll_cost))?],
        visible: [true],
    };
    check_length(shop.items.len())?;
    check_length(
        shop.extra_vouchers
            .len()
            .checked_add(1)
            .ok_or(ObservationError::CollectionTooLarge)?,
    )?;
    check_length(shop.packs.len())?;
    for (index, item) in shop.items.iter().enumerate() {
        let price = Some(item.price(&game.state));
        let row = match item {
            ShopItem::Joker { joker, .. } => {
                encode_joker_values(
                    joker,
                    &game.state.blind,
                    1,
                    index,
                    &mut observation.joker_values,
                )?;
                joker_offer(joker, price, game.state.shop_discount_percent)?
            }
            ShopItem::Consumable {
                consumable,
                price: base_price,
            } => {
                let sell = OwnedConsumable::offer_sell_value(
                    *base_price,
                    game.state.shop_discount_percent,
                );
                consumable_offer(*consumable, price, Some(sell))?
            }
            ShopItem::PlayingCard { card, .. } => card_offer(card, price)?,
        };
        observation.shop_items.push(row)?;
    }
    for voucher in
        std::iter::once(shop.voucher.as_ref()).chain(shop.extra_vouchers.iter().map(Some))
    {
        let row = if let Some(voucher) = voucher {
            Row {
                cat: [ids::vouchers(voucher.voucher)],
                num: [log_number(f64::from(game.state.shop_price(voucher.price)))?],
                cat_visible: [true],
                num_visible: [true],
                present: true,
                ..Row::default()
            }
        } else {
            Row::default()
        };
        observation.shop_vouchers.push(row)?;
    }
    for pack in &shop.packs {
        observation.shop_packs.push(Row {
            cat: [ids::pack_kind(pack.kind), ids::pack_size(pack.size)],
            num: [log_number(f64::from(pack.price(&game.state)))?],
            cat_visible: [true; 2],
            num_visible: [true],
            present: true,
            ..Row::default()
        })?;
    }
    Ok(())
}

fn encode_pack(game: &Game, observation: &mut Observation) -> Result<(), ObservationError> {
    let pending = game
        .pending_pack()
        .ok_or(ObservationError::InvalidState("missing pack"))?;
    let pack = &pending.opening;
    observation.pack = PackObservation {
        cat: [
            ids::pack_kind(pack.kind()),
            ids::pack_size(pack.size()),
            i32::from(pending.previous_state.code()),
        ],
        counts: [integer(pack.choices())?],
        visible: [true; 4],
    };
    check_length(pack.options().len())?;
    for (index, option) in pack.options().iter().enumerate() {
        let row = match option {
            PackOption::Joker(joker) => {
                encode_joker_values(
                    joker,
                    &game.state.blind,
                    2,
                    index,
                    &mut observation.joker_values,
                )?;
                joker_offer(joker, None, game.state.shop_discount_percent)?
            }
            PackOption::Consumable(consumable) => consumable_offer(*consumable, None, None)?,
            PackOption::PlayingCard(card) => card_offer(card, None)?,
        };
        observation.pack_options.push(row)?;
    }
    Ok(())
}

fn joker_offer(
    joker: &Joker,
    price: Option<u16>,
    discount: u8,
) -> Result<Row<10, 5, 5>, ObservationError> {
    if joker.is_face_down() {
        return Err(ObservationError::InvalidState("concealed offer"));
    }
    let mut row = Row {
        present: true,
        ..Row::default()
    };
    row.cat[0] = 1;
    row.cat_visible[0] = true;
    if let Some(price) = price {
        row.num[0] = log_number(f64::from(price))?;
        row.num_visible[0] = true;
    }
    if !joker.is_face_down() {
        row.cat[1] = ids::joker_kind(joker.kind());
        row.cat_visible[1] = true;
        row.cat[8] = ids::joker_edition(joker.edition());
        row.cat_visible[8] = true;
        row.cat[9] = ids::joker_rarity(joker.rarity());
        row.cat_visible[9] = true;
        row.num[1] = log_number(f64::from(joker.sell_value_after_discount(discount)))?;
        row.num_visible[1] = true;
        row.flags = [
            joker.is_debuffed(),
            joker.is_eternal(),
            joker.perishable_rounds().is_some(),
            joker.is_rental(),
            joker.edition() == JokerEdition::Negative,
        ];
        row.flags_visible.fill(true);
    }
    Ok(row)
}

fn consumable_offer(
    consumable: crate::core::enums::Consumable,
    price: Option<u16>,
    sell: Option<u16>,
) -> Result<Row<10, 5, 5>, ObservationError> {
    let [family, identity] = ids::consumable(consumable);
    let mut row = Row {
        present: true,
        ..Row::default()
    };
    row.cat[..3].copy_from_slice(&[2, identity, family]);
    row.cat_visible[..3].fill(true);
    for (index, value) in [price, sell].into_iter().enumerate() {
        if let Some(value) = value {
            row.num[index] = log_number(f64::from(value))?;
            row.num_visible[index] = true;
        }
    }
    row.flags_visible[4] = true;
    Ok(row)
}

fn card_offer(card: &Card, price: Option<u16>) -> Result<Row<10, 5, 5>, ObservationError> {
    if card.is_face_down() {
        return Err(ObservationError::InvalidState("concealed offer"));
    }
    let card_row = hand_row(card, false)?;
    let mut row = Row {
        present: true,
        ..Row::default()
    };
    row.cat[0] = 3;
    row.cat_visible[0] = true;
    row.cat[3..8].copy_from_slice(&card_row.cat);
    row.cat_visible[3..8].copy_from_slice(&card_row.cat_visible);
    row.num[2..].copy_from_slice(&card_row.num);
    row.num_visible[2..].copy_from_slice(&card_row.num_visible);
    row.flags[0] = card_row.flags[1];
    row.flags_visible[0] = card_row.flags_visible[1];
    if let Some(price) = price {
        row.num[0] = log_number(f64::from(price))?;
        row.num_visible[0] = true;
    }
    Ok(row)
}

fn check_length(length: usize) -> Result<(), ObservationError> {
    if length > i32::MAX as usize {
        Err(ObservationError::CollectionTooLarge)
    } else {
        Ok(())
    }
}

fn integer(value: impl TryInto<i64>) -> Result<i64, ObservationError> {
    value
        .try_into()
        .map_err(|_| ObservationError::IntegerOverflow)
}

fn reserve<T>(values: &mut Vec<T>, extra: usize) -> Result<(), ObservationError> {
    values
        .try_reserve(extra)
        .map_err(|_| ObservationError::AllocationFailed)
}

fn raw_number(value: f64) -> Result<f32, ObservationError> {
    let result = value as f32;
    if value.is_finite() && result.is_finite() {
        Ok(if result == 0.0 { 0.0 } else { result })
    } else {
        Err(ObservationError::NonFinite)
    }
}

fn log_number(value: f64) -> Result<f32, ObservationError> {
    if !value.is_finite() {
        return Err(ObservationError::NonFinite);
    }
    raw_number(if value == 0.0 {
        0.0
    } else {
        value.signum() * value.abs().ln_1p()
    })
}

#[cfg(test)]
mod numeric_tests {
    use super::*;
    #[test]
    fn transforms_and_lengths_have_explicit_boundaries() {
        assert_eq!(log_number(0.0), Ok(0.0));
        assert_eq!(log_number(-0.0).unwrap().to_bits(), 0);
        assert_eq!(log_number(-10.0).unwrap(), -log_number(10.0).unwrap());
        assert!(log_number(f64::MAX).unwrap().is_finite());
        for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert_eq!(log_number(value), Err(ObservationError::NonFinite));
        }
        assert_eq!(check_length(i32::MAX as usize), Ok(()));
        assert_eq!(
            check_length(i32::MAX as usize + 1),
            Err(ObservationError::CollectionTooLarge)
        );
        assert_eq!(
            reserve(&mut Vec::<i32>::new(), usize::MAX),
            Err(ObservationError::AllocationFailed)
        );
    }
}
