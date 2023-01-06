/**
 * EAG HTTP REST API for team registration
 * 
 * Features :
 *  - Configuration via the teams.conf file
 *  - Works with attendize
 *  - Verification of individual participants : checks if the participant is not refunded and if he is a athlete
 *  - Team verification and registration
 *  - 
 * 
 * Made with the Rocket Rust Framework
 */

#[macro_use]extern crate rocket;

use rocket_db_pools::{sqlx, Database, Connection};

pub mod config;
pub mod defs;
pub mod checks;

use config::find_sport;
use rocket::serde::json::Json;
use rocket::http::Status;
use rocket::response::{content, status, Redirect};

use defs::*;
use checks::*;

#[derive(Database, Clone)]
#[database("attendize")]
pub struct Attendize(sqlx::MySqlPool);

#[get("/")]
pub fn index() -> Redirect {
    Redirect::to("https://european-aerostudent-games.com")
}

#[get("/sports/<order_ref>")]
pub async fn get_attendee_sports(mut db: Connection<Attendize>, order_ref: &str) -> Option<Json<Vec<Sport>>> {
    let attendee_opt = retrieve_attendee(&mut db, order_ref).await;

    match attendee_opt {
        Some(ida) => return Some(Json(ida.sports)),
        None => return None
    }
}

#[get("/check/<team_sport>/<order_ref>")]
pub async fn check_attendee(mut db: Connection<Attendize>, team_sport: &str, order_ref: &str) -> status::Custom<content::RawJson<&'static str>> {
    let attendee = retrieve_attendee(&mut db, order_ref).await;
    if attendee.is_some()
    {
        let id_attendee = attendee.unwrap();
        let sport = find_sport(team_sport, Some(id_attendee.gender));
        if sport.is_none() {
            return status::Custom(Status::BadRequest, content::RawJson("{\"message\":\"Participant has an invalid sport name or sport is unavailable\"}"));
        }
        match validate_attendee(&mut db, &id_attendee, &sport.unwrap()).await {
            AttendeeStatus::SportNotRegistered => 
                status::Custom(Status::Unauthorized, content::RawJson("{\"message\":\"Participant did not register in the correct sport\"}")),
            AttendeeStatus::InvalidSport => 
                status::Custom(Status::Unauthorized, content::RawJson("{\"message\":\"Participant has an invalid sport name or sport is unavailable\"}")),
            AttendeeStatus::AlreadyInATeam =>
                status::Custom(Status::BadRequest, content::RawJson("{\"message\":\"Participant is already in a team\"}")),
            AttendeeStatus::NotAnAthlete =>
                status::Custom(Status::BadRequest, content::RawJson("{\"message\":\"Participant is a supporter, not an athlete\"}")),
            AttendeeStatus::InvalidGender =>
                status::Custom(Status::Unauthorized, content::RawJson("{\"message\":\"This sport doesn't allow mixed teams, team members should have the same gender\"}")),
            AttendeeStatus::Ok => 
                status::Custom(Status::Ok, content::RawJson("{\"message\":\"Ok\"}")) //TODO: retourner le participant
        }
    }
    else
    {
        status::Custom(Status::NotFound, content::RawJson("{\"message\":\"Attendee not found\"}"))
    }
}

#[post("/create", format="json", data="<team>")]
pub async fn create_team(mut db: Connection<Attendize>, team: Json<Team>) -> status::Custom<content::RawJson<&'static str>> {
    let attendee_gender:Option<AttendeeGender>;
    // Check number of team members
    match team.gender {
        SportGender::M => attendee_gender = Some(AttendeeGender::M),
        SportGender::F => attendee_gender = Some(AttendeeGender::F),
        SportGender::Mixed => attendee_gender = None
    }
    let sport_option = config::find_sport(&team.sport, attendee_gender);
    let sport: &Sport;
    if sport_option.is_none() {
        return status::Custom(Status::NotFound, content::RawJson("{\"message\":\"Invalid \'sport\' field in JSON payload\"}"));
    }
    else {
        sport = sport_option.as_ref().unwrap();
        let nb_members = team.refs.len();
        if nb_members <= usize::from(sport.min_players) || nb_members >= usize::from(sport.max_players) {
            return status::Custom(Status::Unauthorized, content::RawJson("{\"message\":\"Invalid number of team members\"}"));
        }
    }

    // Re-validate attendees
    let mut attendee_list:Vec<IdentifiedAttendee> = Vec::new();
    for reference in &team.refs {
        let attendee = retrieve_attendee(&mut db, reference.as_str()).await;
        if attendee.is_none() {
            return status::Custom(Status::BadRequest, content::RawJson("{\"message\":\"An invalid order reference was sent while creating a team, contact the webmaster\"}"));
        }
        else {
            let id_attendee = attendee.unwrap();
            if validate_attendee(&mut db, &id_attendee, &sport).await != AttendeeStatus::Ok {
                return status::Custom(Status::BadRequest, content::RawJson("{\"message\":\"One of the participants in the team causes a validation error, try re-creating the team and contact the webmaster if this error persists\"}"));
            }
            else {
                attendee_list.push(id_attendee);
            }
        }
    }

    // Create the new team
    let res = sqlx::query("INSERT INTO teams(name, sport) VALUES (?,?)").bind(&team.name).bind(&team.sport).execute(&mut *db).await;
    let team_id:u64;
    if res.is_ok() {
        team_id = res.unwrap().last_insert_id();
    }
    else {
        return status::Custom(Status::InternalServerError, content::RawJson("{\"message\":\"SQL error encountered while creating team, contact the webmaster\"}"));
    }

    // Add team members
    for participant in attendee_list {
        let insert = sqlx::query("INSERT INTO team_members(team_id, attendee_id) VALUES (?, ?)").bind(team_id).bind(participant.id).execute(&mut *db).await;
        if insert.is_err() {
            return status::Custom(Status::InternalServerError, content::RawJson("{\"message\":\"SQL error encountered while adding members to team, contact the webmaster\"}"));
        }
    }

    status::Custom(Status::Ok, content::RawJson("{\"message\":\"Team registered\"}"))
}

#[launch]
fn rocket() -> rocket::Rocket<rocket::Build> {
    rocket::build()
        .attach(Attendize::init())
        .mount("/attendee/", routes![check_attendee, get_attendee_sports])
        .mount("/team/", routes![create_team])
        .mount("/", routes![index])
}