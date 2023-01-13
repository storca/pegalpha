/**
 * EAG HTTP REST API for team registration
 * 
 * Features :
 *  - Configuration via the teams.conf file
 *  - Works with attendize
 *  - Verification of individual participants : checks if the participant is not refunded and if he is a athlete
 *  - Team verification and registration
 * 
 * Made with the Rocket Rust Framework
 */

#[macro_use]extern crate rocket;

use rocket_db_pools::sqlx::Acquire;
use rocket_db_pools::{sqlx, Database, Connection};

pub mod config;
pub mod defs;
pub mod checks;

use config::find_sport;
use rocket::serde::json::Json;
use rocket::response::Redirect;

use defs::*;
use checks::*;

#[derive(Database, Clone)]
#[database("attendize")]
pub struct Attendize(sqlx::MySqlPool);

#[get("/")]
pub fn index() -> Redirect {
    Redirect::to("https://european-aerostudent-games.com")
}

/**
 * ----- API PREFIX /attendee -----
 * 
 * Routes used for information on attendees
 */

#[get("/sports/<order_ref>")]
pub async fn get_attendee_sports(mut db: Connection<Attendize>, order_ref: &str) -> Option<Json<Vec<Sport>>> {
    let attendee_opt = retrieve_attendee(&mut db, order_ref).await;

    match attendee_opt {
        Some(ida) => return Some(Json(ida.sports)),
        None => return None
    }
}

#[get("/check/<team_sport>/<order_ref>")]
pub async fn check_attendee(mut db: Connection<Attendize>, team_sport: &str, order_ref: &str) -> Json<CheckAttendeeResponse> {
    let attendee = retrieve_attendee(&mut db, order_ref).await;
    let mut response = CheckAttendeeResponse {
        message: String::from("Error : unhandled case"),
        attendee: None
    };

    match attendee {
        Some(id_attendee) => {
            let sport = find_sport(team_sport, Some(id_attendee.gender));
            if sport.is_none() {
                response.message = String::from("Participant has an invalid sport name or sport is unavailable");
            }
            match validate_attendee(&mut db, &id_attendee, &sport.unwrap()).await {
                AttendeeStatus::SportNotRegistered => 
                    response.message = String::from("Participant did not register in the correct sport"),
                AttendeeStatus::InvalidSport => 
                    response.message = String::from("Participant has an invalid sport name or sport is unavailable"),
                AttendeeStatus::AlreadyInATeam =>
                    response.message = String::from("Participant is already in a team"),
                AttendeeStatus::NotAnAthlete =>
                    response.message = String::from("Participant is a supporter, not an athlete"),
                AttendeeStatus::InvalidGender =>
                    response.message = String::from("This sport doesn't allow mixed teams, team members should have the same gender"),
                AttendeeStatus::Ok => {
                    response.attendee = Some(id_attendee);
                    response.message = String::from("Ok");
                }
            }
        }
        None => {
            response.message = String::from("Attendee not found");
        }
    }
    return Json(response);
}

/**
 * ----- API PREFIX /team
 * 
 * Routes used to create and retrive information on teams
 */

#[post("/create", format="json", data="<team>")]
pub async fn create_team(mut db: Connection<Attendize>, team: Json<Team>) -> Json<SimpleResponse> {
    let attendee_gender:Option<AttendeeGender>;
    let mut response = SimpleResponse {
        message: String::from("Unhandled case"),
        code: SimpleResponseCode::ServerError
    };

    // Check number of team members
    match team.gender {
        SportGender::M => attendee_gender = Some(AttendeeGender::M),
        SportGender::F => attendee_gender = Some(AttendeeGender::F),
        SportGender::Mixed => attendee_gender = None
    }
    let sport_option = config::find_sport(&team.sport, attendee_gender);
    let sport: &Sport;
    if sport_option.is_none() {
        response.message = String::from("Invalid \'sport\' field in JSON payload");
        response.code = SimpleResponseCode::UserError;
        return Json(response);
    }
    else {
        sport = sport_option.as_ref().unwrap();
        let nb_members = team.refs.len();
        if nb_members <= usize::from(sport.min_players) || nb_members >= usize::from(sport.max_players) {
            response.message = String::from("Invalid number of team members");
            response.code = SimpleResponseCode::UserError;
            return Json(response);
        }
    }

    // Re-validate attendees
    let mut attendee_list:Vec<IdentifiedAttendee> = Vec::new();
    for reference in &team.refs {
        let attendee = retrieve_attendee(&mut db, reference.as_str()).await;
        if attendee.is_none() {
            response.message = String::from("An invalid order reference was sent while creating a team, contact the webmaster");
            response.code = SimpleResponseCode::UserError;
            return Json(response);
        }
        else {
            let id_attendee = attendee.unwrap();
            let status = validate_attendee(&mut db, &id_attendee, &sport).await;
            if  status != AttendeeStatus::Ok {
                response.message = String::from(format!("Participant {reference} causes a validation error, status is {:?}", status));
                response.code = SimpleResponseCode::UserError;
                return Json(response);
            }
            else {
                attendee_list.push(id_attendee);
            }
        }
    }

    // Create the new team
    // Let this be a transaction, because of multiple INSERT statements
    let mut tx = (&mut db).begin().await.unwrap();
    let res = sqlx::query("INSERT INTO teams(name, sport) VALUES (?,?)").bind(&team.name).bind(&team.sport).execute(&mut tx).await;
    let mut team_id:u64 = 0;
    let mut mysql_errored = false;

    match res {
        Ok(r) => team_id = r.last_insert_id(),
        Err(e) => mysql_errored = true
    }

    // Add team members
    if !mysql_errored {
        for participant in attendee_list {
            let insert = sqlx::query("INSERT INTO team_members(team_id, attendee_id) VALUES (?, ?)").bind(team_id).bind(participant.id).execute(&mut tx).await;
            if insert.is_err() {
                mysql_errored = true;
            }
        }
    }

    match mysql_errored {
        true => {
            response.code = SimpleResponseCode::ServerError;
            match tx.rollback().await {
                Ok(_) => response.message = String::from("DB error, rolling back"),
                Err(e) => response.message = String::from(format!("Can\'t roll back : {e}"))
            }
        }
        false => {
            match tx.commit().await {
                Ok(_) => {
                    response.message = String::from("Team created");
                    response.code = SimpleResponseCode::Ok;
                },
                Err(e) => {
                    response.message = String::from(format!("DB commit error : {e}"));
                    response.code = SimpleResponseCode::ServerError;
                }
            }
        }
    }
    return Json(response);
}


#[get("/can_register/<sport_name>", format="json", data="<captain>")]
pub async fn can_register(mut db: Connection<Attendize>, sport_name: &str, captain: Json<IdentifiedAttendee>) -> Json<SimpleResponse>
{
    let mut response = SimpleResponse {
        message: String::from("Unhandled case"),
        code: SimpleResponseCode::ServerError
    };

    let sport_option = config::find_sport(sport_name, Some(captain.gender));
    let sport: Sport;
    if sport_option.is_none() {
        response.message = String::from("Sport not found");
        response.code = SimpleResponseCode::UserError;
    }
    else {
        sport = sport_option.unwrap();
        if sport.allow_multiple_teams {
            response.message = String::from("Ok");
            response.code = SimpleResponseCode::Ok;
        }
        else {
            match school_has_team(&mut *db, &captain, sport_name).await {
                true => response.message = String::from(format!("Your school has already registered a team in this sport, in {sport_name} it is not possible to register multiple teams")),
                false => {
                    response.message = String::from("Ok");
                    response.code = SimpleResponseCode::Ok;
                }
            }
        }
    }
    return Json(response);
}

#[launch]
fn rocket() -> rocket::Rocket<rocket::Build> {
    rocket::build()
        .attach(Attendize::init())
        .mount("/attendee/", routes![check_attendee, get_attendee_sports])
        .mount("/team/", routes![create_team, can_register])
        .mount("/", routes![index])
}