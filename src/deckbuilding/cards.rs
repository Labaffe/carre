use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CardType{
    MainWeapon,
    Passive
}
impl CardType {
    pub fn to_string(&self)->String {
        match self {
            CardType::MainWeapon => "MainWeapon".to_string(),
            CardType::Passive => "Passive".to_string()
        }
    }
}
pub trait Card {
    fn card_type(&self)->CardType;
    fn name(&self)->String;
    fn requirement(&self)->i32;
    fn description(&self)->String;
    fn fmt(&self,f: &mut fmt::Formatter<'_>)->fmt::Result{
        write!(f,"{} - ({}) \n  {} : {}",self.name(),self.requirement(),self.card_type().to_string() ,self.description())
    }
}

pub struct DummyCard;
impl Card for DummyCard {
    fn card_type(&self)->CardType { CardType::Passive }
    fn name(&self)->String { "Dummy card".to_string() }
    fn requirement(&self)->i32 { 1 }
    fn description(&self)->String { "Ceci est une carte inutile. ".to_string() }
}