use rocket::serde::{Serialize, Deserialize};

use rocket_db_pools::sqlx::Row;
use rocket_db_pools::{sqlx};
use rocket_db_pools::sqlx::mysql::MySqlConnection;

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

#[derive(Serialize, Deserialize, Clone, PartialEq)]
#[serde(crate = "rocket::serde")]
pub struct Sport {
    pub name: String,
    pub min_players: u8,
    pub max_players: u8,
    pub gender: SportGender,
    /**
     * How much teams a school is allowed to have in this sport ?
     */
    pub max_teams_per_school: u8
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

#[derive(Serialize, Deserialize, Clone)]
#[serde(crate = "rocket::serde")]
pub struct TeamMember {
    pub first_name: String,
    pub last_name: String,
    pub school: String,
    pub sports: Vec<String>
}

impl TeamMember {
    pub async fn from_indentified_attendee(attendee: &IdentifiedAttendee, db: &mut MySqlConnection) -> TeamMember {
        let attendee_name = sqlx::query("SELECT first_name, last_name FROM attendees WHERE id = ?")
        .bind(attendee.id).fetch_one(&mut *db).await;
        match attendee_name {
            Ok(row) => {
                let school_name_fut = sqlx::query("SELECT name FROM question_options WHERE id = ?")
                .bind(attendee.school_id).fetch_one(&mut *db);

                let mut sports:Vec<String> = vec!();
                for sport in &attendee.sports {
                    sports.push(String::from(&sport.name));
                }

                let tm:TeamMember = TeamMember { 
                    first_name: String::from(row.get::<&str, usize>(0)), 
                    last_name: String::from(row.get::<&str, usize>(1)),
                    school: school_name_fut.await.unwrap().get::<String, usize>(0),
                    sports: sports 
                };
                return tm;
            }
            Err(_) => panic!("Unable to convert IdentifiedAttendee to TeamMember")
        }
    }
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
    pub member: Option<TeamMember>
}