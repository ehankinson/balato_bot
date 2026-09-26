use crate::core::enums::JokerKind;

pub struct Joker {
    kind: JokerKind,
    data: JokerData,
    edition: JokerEdition,
    debuffed: bool,
}