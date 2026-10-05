use crate::core::card::Card;
use crate::core::deck::Deck;
use crate::core::enums::{Decks, Edition, Enhancement, Rank, Seal, Suit};
use rand::rngs::StdRng;
use rand::SeedableRng;

#[test]
fn card_mutations_update_card_identity_and_values() {
    let mut card = Card::new(
        Rank::Seven,
        Suit::Spades,
        Enhancement::None,
        Edition::None,
        Seal::None,
    );
    let initial_id = card.id();

    card.set_rank(Rank::Ace);
    card.set_suit(Suit::Hearts);
    card.set_enhancement(Enhancement::Glass);
    card.set_edition(Edition::Polychrome);
    card.set_seal(Seal::Red);

    assert_eq!(card.rank(), Rank::Ace);
    assert_eq!(card.suit(), Suit::Hearts);
    assert!(matches!(card.enhancement(), Enhancement::Glass));
    assert!(matches!(card.edition(), Edition::Polychrome));
    assert!(matches!(card.seal(), Seal::Red));
    assert_ne!(card.id(), initial_id);
}

#[test]
fn deck_draw_discard_reset_and_remove_operations_mutate_the_deck() {
    let mut rng = StdRng::seed_from_u64(1);
    let mut deck = Deck::new(Decks::Red, &mut rng);

    let drawn = deck.draw_cards(2);
    assert_eq!(drawn.len(), 2);
    assert_eq!(deck.size(), 50);

    deck.discard_cards(drawn);
    assert_eq!(deck.discarded_cards().len(), 2);

    deck.reset_deck();
    assert_eq!(deck.size(), 52);
    assert!(deck.discarded_cards().is_empty());

    assert!(deck.remove_card_at(0).is_some());
    assert_eq!(deck.size(), 51);
    assert!(deck.remove_card_at(100).is_none());
}
