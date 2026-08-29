from logging import raiseExceptions

from sympy.printing.printer import Type

from config.settings import (
    HAND_CONTENT_WIDTH,
    HAND_HEIGHT,
    HAND_WIDTH,
    HAND_X_START_GAP,
    HAND_Y_START_GAP,
)
from core.enums import Edition, Enhancement, Rank, Seal, Suit
from core.models import Card, CardAnnotation, Hand, RenderedHand

# 1445x393 THIS IS THE DIMENSIONS OF the SS WE ARE TAKING FOR HAND SIZE
from core.type_aliases import FEATURE_TYPES, Feature
from rendering.backgrounds import render_background
from rendering.card import render_card
from rendering.layout import (
    calculate_angle,
    calculate_box_dimensions,
    calculate_card_y_lift,
    y_jitter,
)


def calculate_card_gap(card_amount: int, card_width: int) -> float:
    if card_amount <= 1:
        return 0.0

    total_card_space = card_amount * card_width
    shift_space = HAND_CONTENT_WIDTH - total_card_space
    shift_per_card = shift_space / (card_amount - 1)
    return shift_per_card


def render_hand(
    hand: Hand, training: bool = False, training_type: str | None = None
) -> RenderedHand:
    img = render_background(HAND_WIDTH, HAND_HEIGHT, training)

    card_amount = len(hand.cards)
    card_gap: float = 0.0
    annotations: list[CardAnnotation] = []
    if training_type is None:
        training_type = "rank"

    for i, card in enumerate(hand.cards):
        if training_type != "enhancement" and card.enhancement == Enhancement.STONE:
            card.enhancement = Enhancement.NONE
        card_image = render_card(card)

        angle = calculate_angle(i, card_amount)

        if i == 0:
            card_gap = calculate_card_gap(card_amount, card_image.width)

        x_pos = int(HAND_X_START_GAP + i * (card_image.width + card_gap))
        y_pos = round(
            HAND_Y_START_GAP - calculate_card_y_lift(i, card_amount) + y_jitter()
        )

        card_image = card_image.rotate(angle, expand=True)
        img.paste(card_image, (x_pos, y_pos), card_image)

        attribute = getattr(card, training_type)
        annotations.append(
            CardAnnotation(
                card=attribute,
                box=calculate_box_dimensions(
                    card_image, x_pos, y_pos, HAND_WIDTH, HAND_HEIGHT
                ),
            )
        )

    return RenderedHand(image=img, annotations=annotations)


def generate_hand(amount_of_cards: int, feature: Feature | None = None):
    hand = Hand([Card.random() for _ in range(amount_of_cards)])
    training_type = None
    if feature is not None:
        edit_card = hand.cards[0]
        if isinstance(feature, Rank):
            edit_card.rank = feature
            training_type = "rank"
        elif isinstance(feature, Suit):
            edit_card.suit = feature
            training_type = "suit"
        elif isinstance(feature, Seal):
            edit_card.seal = feature
            training_type = "seal"
        elif isinstance(feature, Edition):
            edit_card.edition = feature
            training_type = "edition"
        elif isinstance(feature, Enhancement):
            edit_card.enhancement = feature
            training_type = "enhancement"
        else:
            raise TypeError(f"The type: {type(feature)} can not be edited for cards")

    return render_hand(hand, training=True, training_type=training_type)


def main() -> None:
    hand = Hand.random_hand(11)
    img = render_hand(hand).image
    img.save("out.png")


if __name__ == "__main__":
    main()
