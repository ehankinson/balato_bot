from config.settings import (
    HAND_HEIGHT,
    HAND_WIDTH,
    X_RATIO_GAP,
    Y_RATIO_GAP,
)
from core.models import CardAnnotation, Hand, RenderedHand

# 1445x393 THIS IS THE DIMENSIONS OF the SS WE ARE TAKING FOR HAND SIZE
from rendering.backgrounds import render_background
from rendering.card import render_card
from rendering.layout import (
    calculate_box_dimensions,
    calculate_angle,
    calculate_card_y_lift,
    y_jitter,
)

X_START_GAP = int(X_RATIO_GAP * HAND_WIDTH)
Y_START_GAP = int(Y_RATIO_GAP * HAND_HEIGHT)
TOTAL_HAND_WIDTH_RATIO: float = 1372 / 1445
TOTAL_HAND_WIDTH = TOTAL_HAND_WIDTH_RATIO * HAND_WIDTH



def calculate_card_gap(card_amount: int, card_width: int) -> float:
    if card_amount <= 1:
        return 0.0

    total_card_space = card_amount * card_width
    shift_space = TOTAL_HAND_WIDTH - total_card_space
    shift_per_card = shift_space / (card_amount - 1)
    return shift_per_card





def render_hand(hand: Hand, training: bool = False) -> RenderedHand:
    img = render_background(HAND_WIDTH, HAND_HEIGHT, training)

    card_amount = len(hand.cards)
    card_gap: float = 0.0
    annotations: list[CardAnnotation] = []

    for i, card in enumerate(hand.cards):
        card_image = render_card(card)

        angle = calculate_angle(i, card_amount)

        if i == 0:
            card_gap = calculate_card_gap(card_amount, card_image.width)

        x_pos = int(X_START_GAP + i * (card_image.width + card_gap))
        y_pos = round(Y_START_GAP - calculate_card_y_lift(i, card_amount) + y_jitter())

        card_image = card_image.rotate(angle, expand=True)
        img.paste(card_image, (x_pos, y_pos), card_image)

        annotations.append(CardAnnotation(
            card=card,
            box=calculate_box_dimensions(card_image, x_pos, y_pos, HAND_WIDTH, HAND_HEIGHT)
        ))

    return RenderedHand(
        image=img,
        annotations=annotations
    )



def main() -> None:
    hand = Hand.random_hand(11)
    img = render_hand(hand).image
    img.save("out.png")


if __name__ == '__main__':
    main()
