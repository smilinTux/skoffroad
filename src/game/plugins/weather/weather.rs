// Weather condition enum for the weather system
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Weather {
    Clear,
    Cloudy,
    Rain,
    Storm,
    Fog,
    Snow,
}
