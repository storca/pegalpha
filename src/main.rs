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

use rocket::serde::{Serialize, Deserialize, json::Json};
use rocket::http::Status;
use rocket::response::{content, status, Redirect};
use rocket_db_pools::sqlx::Row;
use rocket_db_pools::sqlx::mysql::MySqlConnection;

use std::env;
use ini::Ini;

use rocket_db_pools::{sqlx, Database, Connection};

#[derive(Database)]
#[database("attendize")]
struct Attendize(sqlx::MySqlPool);

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq)]
#[serde(crate = "rocket::serde")]
enum SportGender {
    M,
    F,
    Mixed
}

#[derive(Serialize, Clone)]
#[serde(crate = "rocket::serde")]
struct Sport {
    name: String,
    min_players: u8,
    max_players: u8,
    gender: SportGender
}

#[derive(Serialize, Clone)]
#[serde(crate = "rocket::serde")]
struct UnidentifiedAttendee {
    order_ref: String,
    sport: String
}

#[derive(Serialize, Clone)]
#[serde(crate = "rocket::serde")]
struct IdentifiedAttendee {
    id: u32,
    ticket_id: u32,
    gender: SportGender,
    sports: Vec<Sport>,
    school_id: u32
}

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
struct Team {
    name: String,
    sport: String,
    refs: Vec<String>,
    gender: SportGender
}

#[derive(PartialEq)]
enum AttendeeStatus {
    Ok,
    InvalidSport,
    InvalidGender,
    SportNotRegistered,
    AlreadyInATeam,
    NotAnAthlete
}


fn get_option(opt_name: &str) -> String {
    let filename = env::var("EAG_API_CONFIG");
    let i: Ini;
    if filename.is_err() {
        i = Ini::load_from_file("sample.conf").unwrap();
    }
    else {
        i = Ini::load_from_file(filename.unwrap()).unwrap();
    }
    for(sec, prop) in i.iter() {
        if sec.unwrap() == "main" {
            let opt = prop.get(opt_name);
            if opt.is_some() {
                let mut s = String::new();
                s.push_str(opt.unwrap());
                return s;
            }
            else {
                panic!("Missing option in configuration file under section [main]: {}", opt_name);
            }
        }
    }
    panic!("Missing section [main] in configuration file");
}

fn find_sport(sport: &str, gender:SportGender) -> Option<Sport> {
    let filename = env::var("EAG_API_CONFIG");
    let i: Ini;
    if filename.is_err() {
        i = Ini::load_from_file("sample.conf").unwrap();
    }
    else {
        i = Ini::load_from_file(filename.unwrap()).unwrap();
    }
    for (sec, prop) in i.iter() {
        let section_name = sec.unwrap();
        if section_name == sport {
            // Check if sport type is strict or mixed
            let sport_type = prop.get("type");
            if sport_type.is_some() {
                match sport_type.unwrap() {
                    //This sport supports mixed teams
                    "mixed" => {
                        let min_opt = prop.get("min");
                        let max_opt = prop.get("max");
                        if min_opt.is_some() && max_opt.is_some() { //if min and max are both defined
                            // convert min and max to i8 and return a structure
                            let s = Sport {
                                name: String::from(section_name),
                                min_players: min_opt.unwrap().parse::<u8>().unwrap(),
                                max_players: max_opt.unwrap().parse::<u8>().unwrap(),
                                gender: SportGender::Mixed
                            };
                            return Some(s);
                        }
                    }
                    //This sport supports only one gender per team
                    "strict" => {
                        match gender {
                            SportGender::M => {
                                let min_opt = prop.get("minM");
                                let max_opt = prop.get("maxM");

                                return Some(Sport { name: String::from(section_name),
                                                    min_players: min_opt.unwrap().parse::<u8>().unwrap(), 
                                                    max_players: max_opt.unwrap().parse::<u8>().unwrap(), 
                                                    gender: SportGender::M });
                            },
                            SportGender::F => {
                                let min_opt = prop.get("minF");
                                let max_opt = prop.get("maxF");

                                return Some(Sport { name: String::from(section_name), 
                                    min_players: min_opt.unwrap().parse::<u8>().unwrap(), 
                                    max_players: max_opt.unwrap().parse::<u8>().unwrap(), 
                                    gender: SportGender::F });
                            },
                            SportGender::Mixed => todo!("Welp")
                        }
                    }
                    // When type option is not valid 
                    &_ => panic!("Invalid sport type under [{}], is has to be either \'mixed\' or \'strict\'", section_name)
                }
            }
        }
    }
    return None;
}

#[get("/<order_ref>")]
async fn get_attendee_sports(mut db: Connection<Attendize>, order_ref: &str) -> Option<Json<Vec<Sport>>> {
    let attendee_opt = retrieve_attendee(&mut db, order_ref).await;

    match attendee_opt {
        Some(ida) => return Some(Json(ida.sports)),
        None => return None
    }
}

async fn retrieve_attendee(db: &mut MySqlConnection, order_ref:&str) -> Option<IdentifiedAttendee> {
    if order_ref.len() > 10
    {
        return None;
    }
    else {
        let iter = order_ref.split('-');
        let split_ref = iter.collect::<Vec<&str>>();
        if split_ref.len() != 2 {
            return None;
        }
        println!("order_ref : {}, index : {}", split_ref[0], split_ref[1]);
        // Retrieve attendee_id, ticket id and gender (one row only)
        let first_stmt = format!("SELECT a.id, a.ticket_id, qa.answer_text
        FROM orders o, attendees a, question_answers qa
        WHERE o.event_id = 2
        AND qa.attendee_id = a.id
        AND qa.question_id = {}
        AND a.is_cancelled = 0
        AND a.ticket_id IN {}
        AND o.id = a.order_id
        AND o.order_reference = ?
        AND a.reference_index = ?", get_option("gender_question_id"), get_option("pack_ticket_ids"));

        let first_res = sqlx::query(&first_stmt).bind(split_ref[0]).bind(split_ref[1])
        .fetch_optional(&mut *db).await;

        if first_res.is_err() {
            panic!("SQL error while retrieving attendee");
        }

        let first_row_opt = first_res.unwrap();
        if first_row_opt.is_none() {
            return None;
        }

        let first_row = first_row_opt.unwrap();

        let attendee_id:u32 = first_row.get(0);
        let ticket_id:u32 = first_row.get(1);
        
        let gender_name:String = first_row.get(2);
        let gender:SportGender;

        match gender_name.as_str() {
            "Male" => gender = SportGender::M,
            "Female" => gender = SportGender::F,
            _ => panic!("Gender name unknown {}", gender_name)
        }

        // Get attendee sports
        let second_stmt = format!(
            "SELECT answer_text FROM question_answers
            WHERE attendee_id = ?
            AND question_id IN {}",
            get_option("sport_question_ids")
        );
        let second_res = sqlx::query(&second_stmt).bind(attendee_id)
        .fetch_all(&mut *db).await;

        if second_res.is_err() {
            panic!("SQL error while retrieving attendee sports");
        }

        let mut sports:Vec<Sport> = Vec::new();

        for row in second_res.unwrap() {
            let sport_name:String = row.get(0);
            
            let sport = find_sport(sport_name.as_str(), gender);
            if sport.is_some() { //Ignore sports that are not in the config file (individual sports)
                sports.push(sport.unwrap());
            }
        }

        let school_stmt: String = format!(
            "SELECT id FROM question_options qo
            JOIN question_answers qa ON qa.question_id = qo.question_id
            WHERE qa.question_id = {} AND qa.attendee_id = ?",
            get_option("school_question_id")
        );

        let school_res = sqlx::query(&school_stmt).bind(attendee_id)
        .fetch_one(&mut *db).await;
        if school_res.is_err() {
            panic!("SQL error while retrieving attendee school");
        }

        return Some(IdentifiedAttendee { 
            id: attendee_id, 
            ticket_id: ticket_id, 
            gender: gender, 
            sports: sports });
    }
}

//Useless
/**
async fn has_sport(db: &mut MySqlConnection, attendee:&IdentifiedAttendee, sport: &str) -> bool {
    let questions_id = get_option("sport_question_ids");

    let stmt = format!("SELECT a.id, qa.question_id, qa.answer_text
    FROM attendees a, question_answers qa
    WHERE qa.attendee_id = ?
    AND qa.question_id IN {}
    AND qa.answer_text = ?", questions_id);

    let req = sqlx::query(&stmt).bind(attendee.id).bind(sport);

    let row = req.fetch_optional(db).await;
    match row {
        Ok(o) => 
            match o {
                Some(_r) => true,
                None => false
            }
        Err(e) => panic!("MySQL error during sport registration test : {e:?}")
    }

} */

fn has_sport(attendee:&IdentifiedAttendee, sport_name:&str) -> bool {
    for sport in &attendee.sports {
        if sport.name.as_str() == sport_name {
            return true;
        }
    }
    return false;
}

fn has_correct_gender(attendee:&IdentifiedAttendee, team_sport:Sport) -> bool {
    for sport in &attendee.sports {
        if sport.gender == team_sport.gender {
            return true;
        }
    }
    return false;
}

async fn has_team(db: &mut MySqlConnection, attendee:&IdentifiedAttendee, sport: &str) -> bool {
    let row = sqlx::query("SELECT t.id, t.name
    FROM teams t, team_members tm
    WHERE tm.team_id = t.id
    AND tm.attendee_id = ?
    AND t.sport = ?").bind(attendee.id).bind(sport)
    .fetch_optional(db).await;

    match row {
        Ok(o) => 
            match o {
                Some(_r) => false,
                None => true
            },
        Err(e) => panic!("MySQL error during team registration test : {e:?}")
    }
}

//TODO: change sport type from &str to SportGender
async fn validate_attendee(db: &mut MySqlConnection, attendee:&IdentifiedAttendee, sport: Sport) -> AttendeeStatus {
    let athlete_tickets = get_option("athlete_ticket_ids");
    let mut is_an_athlete = false;
    for ticket_id in athlete_tickets.split(',') {
        let id = ticket_id.parse::<u32>().unwrap();
        if id == attendee.ticket_id {
            is_an_athlete = true;
        }
    }

    // Check if attendee sports are valid
    if attendee.sports.len() == 0 {
        AttendeeStatus::InvalidSport
    }
    else if !is_an_athlete {
        AttendeeStatus::NotAnAthlete
    }
    // Check if attendee is already in a team
    else if has_team(db, attendee, sport.name.as_str()).await {
        AttendeeStatus::AlreadyInATeam
    }
    else if !has_sport(attendee, sport.name.as_str()){
        AttendeeStatus::InvalidSport
    }
    else if !has_correct_gender(attendee, sport) {
        AttendeeStatus::InvalidGender
    }
    else {
        AttendeeStatus::Ok
    }
}

#[get("/check-attendee/<team_sport>/<team_gender>/<order_ref>")]
async fn check_attendee(mut db: Connection<Attendize>, team_sport: &str, team_gender: &str, order_ref: &str) -> status::Custom<content::RawJson<&'static str>> {
    let attendee = retrieve_attendee(&mut db, order_ref).await;
    if attendee.is_some()
    {
        let id_attendee = attendee.unwrap();
        match validate_attendee(&mut db, &id_attendee, team_sport).await {
            AttendeeStatus::SportNotRegistered => 
                status::Custom(Status::Unauthorized, content::RawJson("{\"message\":\"Participant did not register in the correct sport\"}")),
            AttendeeStatus::InvalidSport => 
                status::Custom(Status::BadRequest, content::RawJson("{\"message\":\"Participant has an invalid sport name or sport is unavailable\"}")),
            AttendeeStatus::AlreadyInATeam =>
                status::Custom(Status::BadRequest, content::RawJson("{\"message\":\"Participant is already in a team\"}")),
            AttendeeStatus::NotAnAthlete =>
                status::Custom(Status::BadRequest, content::RawJson("{\"message\":\"Participant is a supporter, not an athlete\"}")),
            AttendeeStatus::InvalidGender =>
                status::Custom(Status::Unauthorized, content::RawJson("{\"message\":\"This sport doesn't allow mixed teams, team members should have the same gender\"}")),
            AttendeeStatus::Ok => 
                status::Custom(Status::Ok, content::RawJson("{\"message\":\"Ok\"}"))
        }
    }
    else
    {
        status::Custom(Status::NotFound, content::RawJson("{\"message\":\"Attendee not found\"}"))
    }
}

#[post("/create-team", format="json", data="<team>")]
async fn create_team(mut db: Connection<Attendize>, team: Json<Team>) -> status::Custom<content::RawJson<&'static str>> {
    // Check number of team members
    let s = find_sport(&team.sport, team.gender);
    if s.is_none() {
        return status::Custom(Status::NotFound, content::RawJson("{\"message\":\"Invalid \'sport\' field in JSON payload\"}"));
    }
    else {
        let sport = s.unwrap();
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
            if validate_attendee(&mut db, &id_attendee, &team.sport).await != AttendeeStatus::Ok {
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

#[get("/")]
fn index() -> Redirect {
    Redirect::to("https://european-aerostudent-games.com")
}

#[launch]
fn rocket() -> rocket::Rocket<rocket::Build> {    
    rocket::build()
        .attach(Attendize::init())
        .mount("/sports", routes![get_attendee_sports])
        .mount("/", routes![check_attendee, create_team, index])
}
