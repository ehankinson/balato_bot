from core.enums import Edition, JokersName


JOKER_TYPE_CLASSES = tuple(
    joker for joker in JokersName
    if "REAL_FACE" not in joker.name
)
JOKER_TYPE_CLASS_IDS = tuple(int(joker) for joker in JOKER_TYPE_CLASSES)
JOKER_TYPE_CLASS_NAMES = tuple(str(joker_id) for joker_id in JOKER_TYPE_CLASS_IDS)

NEGATIVE_JOKER_EDITION_ID = 4
JOKER_EDITION_CLASS_IDS = tuple(int(edition) for edition in Edition) + (NEGATIVE_JOKER_EDITION_ID,)
JOKER_EDITION_CLASS_NAMES = tuple(str(edition_id) for edition_id in JOKER_EDITION_CLASS_IDS)
