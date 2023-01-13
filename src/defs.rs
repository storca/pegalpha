use rocket::serde::{Serialize, Deserialize};

/**
 * ------ Type Definitions
 */

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

#[derive(Serialize, Deserialize, Clone)]
#[serde(crate = "rocket::serde")]
pub struct Sport {
    pub name: String,
    pub min_players: u8,
    pub max_players: u8,
    pub gender: SportGender,
    /**
     * Is a school allowed to have multiple teams in this sport ?
     * by default, yes
     */
    pub allow_multiple_teams: bool
}

#[derive(Serialize, Clone)]
#[serde(crate = "rocket::serde")]
pub struct UnidentifiedAttendee {
    pub order_ref: String,
    pub sport: String
}

#[derive(Serialize, Deserialize, Clone)]
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

#[derive(PartialEq, Debug)]
pub enum AttendeeStatus {
    Ok,
    InvalidSport,
    InvalidGender,
    SportNotRegistered,
    AlreadyInATeam,
    NotAnAthlete
}

/**
 * ------- Reponse definitions
 */

#[derive(Serialize, Clone)]
#[serde(crate = "rocket::serde")]
 pub enum SimpleResponseCode {
    Ok,
    UserError,
    ServerError
}

#[derive(Serialize, Clone)]
#[serde(crate = "rocket::serde")]
pub struct SimpleResponse {
    pub message: String,
    pub code: SimpleResponseCode
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(crate = "rocket::serde")]
pub struct CheckAttendeeResponse {
    pub message: String,
    pub attendee: Option<IdentifiedAttendee>
}