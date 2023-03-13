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

use std::path::{Path, PathBuf};

use rocket_db_pools::sqlx::Acquire;
use rocket_db_pools::{sqlx, Database, Connection};

pub mod config;
pub mod defs;
pub mod checks;

use config::{find_sport, get_option};
use rocket::serde::json::Json;
use rocket::response::Redirect;
use rocket::fs::NamedFile;
use rocket::Request;

use rocket_dyn_templates::{Template, context};

use defs::*;
use checks::*;

use rocket_db_pools::sqlx::Row;

#[derive(Database, Clone)]
#[database("attendize")]
pub struct Attendize(sqlx::MySqlPool);

/**
 * API routes
 */

/**
 * ----- API PREFIX /attendee -----
 * 
 * Routes used for information on attendees
 */

#[get("/attendee/sports/<order_ref>")]
pub async fn get_attendee_sports(mut db: Connection<Attendize>, order_ref: &str) -> Option<Json<Vec<Sport>>> {
    let attendee_opt = retrieve_attendee(&mut db, order_ref).await;

    match attendee_opt {
        Ok(ida) => {
            return Some(Json(ida.sports));
        }
        Err(_) => return None
    }
}
/**
#[get("/attendee/<order_ref>")]
pub async fn get_attendee(mut db: Connection<Attendize>, order_ref:&str) -> Option<Json<IdentifiedAttendee>> {
    match retrieve_attendee(&mut db, order_ref).await {
        Ok(a) => Some(Json(a)),
        Err(_) => None
    }
} */

#[get("/attendee/check/<team_sport>/<team_gender>/<order_ref>")]
pub async fn get_check_attendee(mut db: Connection<Attendize>, team_sport: &str, team_gender: &str, order_ref: &str) -> Json<CheckAttendeeResponse> {
    let mut response = CheckAttendeeResponse {
        message: String::from("Error : unhandled case"),
        member: None
    };

    let attendee = retrieve_attendee(&mut db, order_ref).await;

    match attendee {
        Ok(id_attendee) => {
            let gender: AttendeeGender;
            match team_gender {
                "M" => gender = AttendeeGender::M,
                "Mixed" => gender = id_attendee.gender,
                "F" => gender = AttendeeGender::F,
                _other => {
                    response.message = String::from("Invalid gender option");
                    return Json(response);
                }
            }

            let sport = find_sport(team_sport, Some(gender));
            if sport.is_err() {
                response.message = String::from("Participant has an invalid sport name or sport is unavailable");
                return Json(response);
            }

            let m = TeamMember::from_identified_attendee(&id_attendee, &mut *db).await;
            let fullname = format!("{} {}", m.first_name, m.last_name);

            match validate_attendee(&mut db, &id_attendee, &sport.unwrap()).await {
                AttendeeStatus::SportNotRegistered => 
                    response.message = format!("{fullname} did not register in {team_sport}"),
                AttendeeStatus::InvalidSport => 
                    response.message = format!("{fullname} has an invalid sport name or sport is unavailable"),
                AttendeeStatus::AlreadyInATeam =>
                    response.message = format!("{fullname} is already in a {team_sport} team"),
                AttendeeStatus::NotAnAthlete =>
                    response.message = format!("{fullname} is a supporter, not an athlete"),
                AttendeeStatus::InvalidGender =>
                    response.message = String::from(format!("{team_sport} does not allow mixed teams, every team member must be of gender {team_gender}")),
                AttendeeStatus::Ok => {
                    response.member = Some(TeamMember::from_identified_attendee(&id_attendee, &mut *db).await);
                    response.message = String::from("Ok");
                }
            }
        }
        Err(_) => {
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

#[post("/team/create", format="json", data="<team>")]
pub async fn post_create_team(mut db: Connection<Attendize>, team: Json<Team>) -> Json<SimpleResponse> {
    let attendee_gender:Option<AttendeeGender>;
    let mut response = SimpleResponse {
        message: String::from("Unhandled case"),
        code: SimpleResponseCode::ServerError
    };
    match team_registration_open() {
        Ok(_) => (),
        Err(_) => {
            response.message = format!("It is not possible to register a team right now");
            response.code = SimpleResponseCode::UserError;
            return Json(response);
        }
    }
    // Check number of team members
    match team.gender {
        SportGender::M => attendee_gender = Some(AttendeeGender::M),
        SportGender::F => attendee_gender = Some(AttendeeGender::F),
        SportGender::Mixed => attendee_gender = None
    }
    let sport:Sport;
    match config::find_sport(&team.sport, attendee_gender) {
        Ok(s) => {
            sport = s;
            let nb_members = team.refs.len();
            if nb_members < usize::from(sport.min_players) || nb_members > usize::from(sport.max_players) {
                let min = sport.min_players;
                let max = sport.max_players;
                response.message = format!("Invalid number of team members, is should be between {min} and {max} and not {nb_members}");
                response.code = SimpleResponseCode::UserError;
                return Json(response);
            }
        }
        Err(_e) => {
            response.message = String::from("Invalid \'sport\' field in JSON payload");
            response.code = SimpleResponseCode::UserError;
            return Json(response);
        }
    }

    let attendee_list:Vec<IdentifiedAttendee>;
    match validate_team(&mut *db, &team, sport).await {
        Ok(al) => attendee_list = al,
        Err(e) => {
            response.message = e;
            response.code = SimpleResponseCode::UserError;
            return Json(response);
        }
    }

    let captain_id = attendee_list[0].id;

    // Create the new team
    // Let this be a transaction, because of multiple INSERT statements
    let mut tx = (&mut *db).begin().await.unwrap();
    let res = sqlx::query("INSERT INTO teams(school_id, name, captain_id, uuid, sport, gender) VALUES (?,?,?,UUID(),?,?)")
        .bind(&team.school_id)
        .bind(&team.name)
        .bind(captain_id)
        .bind(&team.sport)
        .bind(
            match &team.gender {
                SportGender::M => "Male",
                SportGender::F => "Female",
                SportGender::Mixed => "Mixed"
            }
        )
        .execute(&mut tx).await;
    let mut team_id:u64 = 0;
    let mut mysql_errored = false;

    match res {
        Ok(r) => team_id = r.last_insert_id(),
        Err(_e) => mysql_errored = true
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


#[get("/team/can_register/<sport_name>/<order_ref>")]
pub async fn get_can_register(mut db: Connection<Attendize>, sport_name: &str, order_ref: &str) -> Json<SimpleResponse>
{
    let mut response = SimpleResponse {
        message: String::from("Unhandled case"),
        code: SimpleResponseCode::ServerError
    };

    let captain: IdentifiedAttendee;
    match retrieve_attendee(&mut *db, order_ref).await {
        Ok(a) => captain = a,
        Err(_) => {
            response.message = String::from("Attendee not found");
            response.code = SimpleResponseCode::UserError;
            return Json(response);
        }
    }

    match config::find_sport(sport_name, Some(captain.gender)) {
        Ok(sport) => {
            match can_school_register_team(&mut *db, &captain, &sport).await {
                true => {
                    response.message = String::from("Ok");
                    response.code = SimpleResponseCode::Ok;
                }
                false => {
                    response.message = String::from(format!(
                        "Your school has already registered {max_teams_per_school} teams in this sport, in {sport_name} it is not possible to register more teams", 
                        max_teams_per_school = sport.max_teams_per_school, 
                        sport_name = sport.name)
                    );
                    response.code = SimpleResponseCode::UserError;
                    return Json(response);
                }
            }
            match validate_attendee(&mut *db, &captain, &sport).await {
                AttendeeStatus::Ok => {
                    response.message = String::from("Ok");
                    response.code = SimpleResponseCode::Ok;
                }
                other => {
                    response.message = format!("You cannot register a team, code:{:?}", other);
                    response.code = SimpleResponseCode::UserError;
                }
            }
        }
        Err(e) => {
            warn!("{}", e);
            response.message = String::from("Sport not found");
            response.code = SimpleResponseCode::UserError;
        }
    }
    return Json(response);
}

/**
 * Web routes
 */

pub fn team_registration_open() -> Result<(), Template> {
    let val:bool = config::get_option("team_registration_open").parse().unwrap();
    match val {
        true => Ok(()),
        false => Err(Template::render("error", context!{message:"It is currently not possible to register a team"}))
    }
}

#[get("/")]
pub fn get_index() -> Redirect {
    Redirect::to("https://european-aerostudent-games.com")
}

#[get("/static/<path..>")]
pub async fn get_ressource(path: PathBuf) -> Option<NamedFile> {
    NamedFile::open(Path::new("ressources/").join(path)).await.ok()
}

#[get("/welcome/<order_ref>")]
pub async fn get_welcome(mut db: Connection<Attendize>, order_ref: &str) -> Option<Template> {
    match team_registration_open() {
        Ok(_) => (),
        Err(t) => return Some(t)
    }
    match retrieve_attendee(&mut *db, order_ref).await {
        Ok(attendee) => {
            let context = context! {sports: attendee.sports, order_ref: order_ref};
            Some(Template::render("welcome", &context))
        }
        Err(_) => None
    }
}

/**
 * ------ Team routes ------
 */

/**
 * Page where user compose their team
 */
#[get("/compose/<order_ref>/<sport_name>")]
pub async fn get_compose(mut db: Connection<Attendize>, order_ref: &str, sport_name: &str) -> Option<Template> {
    match team_registration_open() {
        Ok(_) => (),
        Err(t) => return Some(t)
    }

    match retrieve_attendee(&mut *db, order_ref).await {
        Ok(id_attendee) => {
            match find_sport(sport_name, Some(id_attendee.gender)) {
                Ok(sport) => {
                    match validate_attendee(&mut *db, &id_attendee, &sport).await {
                        AttendeeStatus::Ok => {
                            let context = context! {
                                captain: TeamMember::from_identified_attendee(&id_attendee, &mut *db).await,
                                sport: sport,
                                captain_ref: order_ref,
                                school_id: id_attendee.school_id
                            };
                            Some(Template::render("compose_team", &context))
                        },
                        _ => None
                    }
                }
                Err(_) => None
            }
        },
        Err(_) => None
    }
}

#[get("/test/<sport_name>")]
pub async fn get_test(sport_name: &str) -> Option<Template> {
    let mut mock_data: Vec<TeamMember> = vec!();
    for i in 1..10 {
        mock_data.push(
            TeamMember { 
                attendee_id: 1,
                first_name: String::from(format!("first name {i}")), 
                last_name: String::from(format!("last name {i}")), 
                school: String::from("School"),
                sports: vec![String::from("Beach volley"), String::from("Rugby")] 
            }
        );
    }
    match find_sport(sport_name, Some(AttendeeGender::F)) {
        Ok(s) => {
            let context = context! {captain: &mock_data[0], sport:s};
            Some(Template::render("compose_team", &context))
        }
        Err(e) => {
            warn!("{}", e);
            None
        }
    }
}

#[get("/success")]
pub async fn get_team_success() -> Template {
    Template::render("success", context!{message:"You're all set, your team was registered!"})
}

#[get("/help")]
pub async fn get_team_help() -> Template {
    Template::render("help", context!{})
}

#[catch(500)]
fn internal_error() -> Json<SimpleResponse> {
    Json(SimpleResponse {
        message: String::from("Whoops! Looks like we messed up."),
        code: SimpleResponseCode::ServerError
    })
}


#[catch(404)]
fn not_found(req: &Request) -> Json<SimpleResponse> {
    Json(SimpleResponse{
        message: format!("I couldn't find '{}'. Try something else?", req.uri()),
        code: SimpleResponseCode::UserError
    })
}

/**
 * ----- TEAM PREVIEW ----------
 */
#[get("/teams/<secret>?<school>&<sport>")]
pub async fn get_list_teams(mut db: Connection<Attendize>, secret:&str, school:Option<u32>, sport:Option<String>) -> Option<Template> {
    let cfg_secret = get_option("secret");
    if cfg_secret.as_str() != secret {
        return None;
    }
    let res:Result<Vec<rocket_db_pools::sqlx::mysql::MySqlRow>, rocket_db_pools::sqlx::Error>;
    if school.is_some() && sport.is_some() {
        res = sqlx::query(
            "SELECT qo.name school, t.name, t.sport, t.gender, t.uuid
            FROM teams t JOIN question_options qo ON t.school_id = qo.id 
            WHERE school_id = ? AND sport = ?
            ORDER BY school"
        )
        .bind(school.unwrap())
        .bind(sport.unwrap())
        .fetch_all(&mut *db)
        .await;
    }
    else if school.is_some() {
        res = sqlx::query(
            "SELECT qo.name school, t.name, t.sport, t.gender, t.uuid
            FROM teams t JOIN question_options qo ON t.school_id = qo.id 
            WHERE school_id = ?
            ORDER BY school"
        )
        .bind(school.unwrap())
        .fetch_all(&mut *db)
        .await;
    }
    else if sport.is_some() {
        res = sqlx::query(
            "SELECT qo.name school, t.name, t.sport, t.gender, t.uuid
            FROM teams t JOIN question_options qo ON t.school_id = qo.id 
            WHERE sport = ?
            ORDER BY school"
        )
        .bind(sport.unwrap())
        .fetch_all(&mut *db)
        .await;
    }
    else {
        res = sqlx::query(
            "SELECT qo.name school, t.name, t.sport, t.gender, t.uuid
            FROM teams t JOIN question_options qo ON t.school_id = qo.id 
            ORDER BY school"
        )
        .fetch_all(&mut *db)
        .await;
    }
    let sports_fut = sqlx::query(
        "SELECT name FROM question_options WHERE question_id IN (5,6,8) ORDER BY name"
    ).fetch_all(&mut *db);
    match res {
        Ok(rows) => {
            let mut teams:Vec<TeamView> = vec![];
            for row in rows {
                teams.push(TeamView { 
                    name: row.get(1), 
                    school: row.get(0), 
                    sport: row.get(2), 
                    gender: row.get(3), 
                    uuid: row.get(4) });
            }
            let mut sports:Vec<String> = vec![];
            for row in sports_fut.await.ok()? {
                sports.push(row.get(0));
            }
            let ctx = context!{teams: teams, sports: sports};
            return Some(Template::render("team_list", &ctx));
        },
        Err(_) => {
            return None;
        }
    }
}

#[get("/no-team/list/<secret>")]
pub async fn get_no_team_list(mut db: Connection<Attendize>, secret: &str) -> Option<Template> {
    let cfg_secret = get_option("secret");
    if cfg_secret.as_str() != secret {
        return None;
    }
    let sports_fut = sqlx::query(
        "SELECT name FROM question_options WHERE question_id IN (5,6,8) ORDER BY name"
    )
    .fetch_all(&mut *db);

    let mut sports:Vec<String> = vec![];
    for row in sports_fut.await.ok()? {
        sports.push(row.get(0));
    }
    Some(
        Template::render("no_team_list", context!{sports: sports, secret: secret})
    )
}

#[get("/no-team/members/<sport>/<secret>")]
pub async fn get_no_team(mut db: Connection<Attendize>, secret: &str, sport: &str) -> Option<Template> {
    let cfg_secret = get_option("secret");
    if cfg_secret.as_str() != secret {
        return None;
    }
    let mut members:Vec<CompleteTeamMember> = vec![];
    let members_qry = sqlx::query(
        "SELECT a.id, a.first_name, a.last_name, a.email, qb.answer_text school, qc.answer_text phone FROM attendees a
        JOIN question_answers qa ON qa.attendee_id = a.id
        JOIN question_answers qb ON qb.attendee_id = a.id
        JOIN question_answers qc ON qc.attendee_id = a.id
        WHERE a.event_id = 2 AND a.is_cancelled = 0
        AND qa.question_id IN (5, 6, 7, 8) AND qa.answer_text = ?
        AND qb.question_id = 15
        AND a.id NOT IN (SELECT attendee_id FROM team_members)
        AND qc.question_id = 4
        ORDER BY school"
    )
    .bind(sport)
    .fetch_all(&mut *db)
    .await
    .ok()?;
    
    for r in members_qry {
        let mut member = CompleteTeamMember {
            attendee_id: r.get(0),
            first_name: r.get(1),
            last_name: r.get(2),
            school: r.get(4),
            sports: vec![],
            email: r.get(4),
            phone: r.get(5)
        };
        let sports = sqlx::query(
            "SELECT answer_text FROM question_answers WHERE attendee_id = ? AND question_id IN (5, 6, 7, 8)"
        )
        .bind(member.attendee_id)
        .fetch_all(&mut *db)
        .await
        .ok()?;
        
        for r in sports {
            member.sports.push(r.get(0));
        }
        members.push(member);
    }
    Some(
        Template::render("view_team", context!{members: members, name: "No team", sport: sport, gender: ""})
    )
}

#[get("/team/<uuid>")]
pub async fn get_team(mut db: Connection<Attendize>, uuid:&str) -> Option<Template> {
    let row = sqlx::query(
        "SELECT id, name, sport, gender FROM teams WHERE uuid=?"
    )
    .bind(uuid)
    .fetch_one(&mut *db).await.ok()?;

    let team_id:u32 = row.get(0);
    let name:String = row.get(1);
    let sport:String = row.get(2);
    let gender:String = row.get(3);

    let rows = sqlx::query(
        "SELECT attendee_id FROM team_members WHERE team_id=?"
    )
    .bind(team_id)
    .fetch_all(&mut *db).await.ok()?;

    let mut members:Vec<CompleteTeamMember> = vec![];

    for row in rows {
        match get_attendee(&mut *db, row.get(0)).await {
            // Only add valid attendees
            Ok(ida) => {
                let member = TeamMember::from_identified_attendee(&ida, &mut *db).await;
                let full_member = CompleteTeamMember::from_team_member(&mut *db, &member).await;
                members.push(full_member);
            },
            Err(_) => ()
        }
    }
    Some(Template::render("view_team", context!{members: members, name, sport, gender}))
}

#[get("/shotgun/<order_ref>?<choice>")]
pub async fn get_shotgun(mut db: Connection<Attendize>, order_ref: &str, choice: Option<bool>) -> Option<Template> {
    // Check if the number of Cross Country participants is < 300
    let nb:i64 = sqlx::query(
        "SELECT COUNT(*) FROM question_answers qa
        JOIN attendees a ON qa.attendee_id = a.id
        WHERE a.event_id = 2 AND qa.question_id = 8 AND qa.answer_text = ?"
    )
    .bind("Cross Country")
    .fetch_one(&mut *db).await.ok()?.get(0);

    if nb > 150 {
        return Some(
            Template::render("error", context!{message:"We have reached the maximum nummber of participants for Cross Country"})
        );
    }

    let id_attendee = retrieve_attendee(&mut *db, order_ref).await.ok()?;

    let athlete_tickets = config::get_option("athlete_ticket_ids");
    let mut is_an_athlete = false;
    for ticket_id in athlete_tickets.split(',') {
        let id = ticket_id.parse::<u32>().unwrap();
        if id == id_attendee.ticket_id {
            is_an_athlete = true;
        }
    }
    if !is_an_athlete {
        return None;
    }

    let row = sqlx::query(
        "SELECT COUNT(*) FROM question_answers WHERE attendee_id = ? AND question_id = 8 AND answer_text = ?"
    )
    .bind(id_attendee.id)
    .bind("Cross Country")
    .fetch_one(&mut *db).await.ok()?;

    let count:i64 = row.get(0);
    if count > 0 {
        return None;
    }

    match choice {
        Some(v) => {
            if v {
                let res = sqlx::query(
                    "INSERT INTO question_answers(attendee_id, event_id, question_id, account_id, answer_text)
                    VALUES (?, 2, 8, 1, ?)"
                )
                .bind(id_attendee.id)
                .bind("Cross Country")
                .execute(&mut *db).await;
                
                if res.is_err() {
                    panic!("MYSQL insert error during shotgun");
                }

                Some(
                    Template::render("success", context!{message: "Your registration to Cross Country has been taken into account"})
                )
            }
            else {
                None
            }
        }
        None => {
            Some(
                Template::render("shotgun", context!{})
            )
        }
    }
}


#[launch]
fn rocket() -> rocket::Rocket<rocket::Build> {
    rocket::build()
        .attach(Attendize::init())
        .attach(Template::fairing())
        .mount("/api/", routes![ 
            get_check_attendee, 
            get_attendee_sports, 
            post_create_team, 
            get_can_register
        ])
        .mount("/", routes![
            get_index, 
            get_ressource, 
            get_welcome,
            get_shotgun
        ])
        .mount("/team", routes![
            get_test,
            get_compose,
            get_team_success,
            get_team_help
        ])
        .mount("/view", routes![
            get_list_teams,
            get_team
        ])
        .register("/api", catchers![not_found, internal_error])
}
