use rocket::serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq)]
#[serde(crate = "rocket::serde")]
pub enum SportGender {
    M,
    F,
    Mixed
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq)]
#[serde(crate = "rocket::serde")]
pub enum AttendeeGender {
    M,
    F
}

#[derive(Serialize, Clone)]
#[serde(crate = "rocket::serde")]
pub struct Sport {
    pub name: String,
    pub min_players: u8,
    pub max_players: u8,
    pub gender: SportGender
}

#[derive(Serialize, Clone)]
#[serde(crate = "rocket::serde")]
pub struct UnidentifiedAttendee {
    pub order_ref: String,
    pub sport: String
}

#[derive(Serialize, Clone)]
#[serde(crate = "rocket::serde")]
pub struct IdentifiedAttendee {
    pub id: u32,
    pub ticket_id: u32,
    pub gender: AttendeeGender,
    pub sports: Vec<Sport>,
    pub school_id: u32
}

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct Team {
    pub name: String,
    pub school_id: u32,
    pub sport: String,
    pub refs: Vec<String>,
    pub gender: SportGender
}

#[derive(PartialEq)]
pub enum AttendeeStatus {
    Ok,
    InvalidSport,
    InvalidGender,
    SportNotRegistered,
    AlreadyInATeam,
    NotAnAthlete
}