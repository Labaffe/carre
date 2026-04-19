use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CardType{
    Primary,
    Secondary,
    Passive
}
impl CardType {
    pub fn to_string(&self)->String {
        match self {
            CardType::Primary => "Primary".to_string(),
            CardType::Secondary => "Secondary".to_string(),
            CardType::Passive => "Passive".to_string()
        }
    }
}

#[derive(Clone)]
pub struct Card {
    pub card_type:CardType,
    pub name:String,
    pub requirement:i32,
    pub description:String,
}
impl Card {
    pub fn new()->Card {
        Card {
            card_type: {
                let i = fastrand::usize(..3);
                if i == 0 { 
                    CardType::Passive 
                }
                else {
                    if i == 1 {
                        CardType::Primary 
                    }
                    else {
                        CardType::Secondary 
                    }
                }
            },
            name: { "Dummy card".to_string() },
            requirement: { 1 },
            description: { "Ceci est une carte inutile. ".to_string() }
        }
    }
}