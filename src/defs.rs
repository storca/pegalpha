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
    pub max_teams_per_school: u8,
    pub school_mix_allowed: bool
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
    pub attendee_id: u32,
    pub first_name: String,
    pub last_name: String,
    pub school: String,
    pub sports: Vec<String>
}

impl TeamMember {
    pub async fn from_identified_attendee(attendee: &IdentifiedAttendee, db: &mut MySqlConnection) -> TeamMember {
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
                    attendee_id: attendee.id,
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

#[derive(Serialize, Deserialize, Clone)]
#[serde(crate = "rocket::serde")]
pub struct CompleteTeamMember {
    pub attendee_id: u32,
    pub first_name: String,
    pub last_name: String,
    pub gender: String,
    pub school: String,
    pub sports: Vec<String>,
    pub email: String,
    pub phone: String,
    pub attendee_ref: String
}

impl CompleteTeamMember {
    pub async fn from_team_member(db: &mut MySqlConnection, member: &TeamMember) -> CompleteTeamMember {
        let result = sqlx::query(
            "SELECT a.email, qa.answer_text, qb.answer_text,
            CONCAT(o.order_reference, '-', a.reference_index) attendee_ref
            FROM attendees a
            JOIN orders o ON a.order_id = o.id
            JOIN question_answers qa ON qa.attendee_id = a.id
            JOIN question_answers qb ON qb.attendee_id = a.id
            WHERE qa.question_id = 3 AND qb.question_id = 17 AND a.id = ?"
        )
        .bind(member.attendee_id)
        .fetch_one(&mut *db).await;
        let email:String;
        let phone:String;
        let attendee_ref:String;
        let gender:String;
        match result {
            Ok(r) => {
                email = r.get(0);
                phone = r.get(1);
                attendee_ref = r.get(3);
                gender = r.get(2);
            }
            Err(_) => {
                email = format!("none");
                phone = format!("none");
                attendee_ref = format!("none");
                gender = format!("none");
            }
        }
        CompleteTeamMember {
            attendee_id: member.attendee_id,
            first_name: member.first_name.clone(),
            last_name: member.last_name.clone(),
            gender: gender,
            school: member.school.clone(),
            sports: member.sports.clone(),
            email: email,
            phone: phone,
            attendee_ref: attendee_ref
        }
    }
    pub async fn from_attendee_id(db: &mut MySqlConnection, attendee_id:u32) -> Option<CompleteTeamMember> {
        let r = sqlx::query(
            "SELECT a.first_name, a.last_name, a.email, qa.answer_text phone, qb.answer_text gender, qc.answer_text school,
            CONCAT(o.order_reference, '-', a.reference_index) attendee_ref
            FROM attendees a
            JOIN orders o ON a.order_id = o.id
            JOIN question_answers qa ON qa.attendee_id = a.id
            JOIN question_answers qb ON qb.attendee_id = a.id
            JOIN question_answers qc ON qc.attendee_id = a.id
            WHERE qa.question_id = 4 AND qb.question_id = 17 AND qc.question_id = 15
            AND a.id = ?"
        )
        .bind(attendee_id)
        .fetch_one(&mut *db)
        .await
        .ok()?;

        let mut member = CompleteTeamMember {
            attendee_id: attendee_id,
            first_name: r.get(0),
            last_name: r.get(1),
            gender: r.get(4),
            school: r.get(5),
            sports: vec![],
            email: r.get(2),
            phone: r.get(3),
            attendee_ref: r.get(6)
        };

        let sports = sqlx::query(
            "SELECT DISTINCT(answer_text) FROM question_answers WHERE attendee_id = ? AND question_id IN (5, 6, 7, 8)"
        )
        .bind(member.attendee_id)
        .fetch_all(&mut *db)
        .await
        .ok()?;
        
        for row in sports {
            member.sports.push(row.get(0));
        }

        Some(member)
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
#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct TeamView {
    pub name: String,
    pub school: String,
    pub sport: String,
    pub gender: String,
    pub uuid: String
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
    pub member: Option<CompleteTeamMember>
}