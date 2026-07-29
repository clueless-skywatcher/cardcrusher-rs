use crate::ids::CardId;

pub struct ChainLink {
    pub effect_seq: usize,
    pub card: CardId,
    pub activator: usize,
    pub targets: Vec<CardId>,
}
